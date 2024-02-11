use std::collections::HashMap;
use clap::{Parser};
use std::{fs, io};
use std::io::Write;
use std::sync::Arc;
use crate::binder::intermediate::collection::Collection;
use crate::binder::Binder;
use crate::binder::json::jsonify;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::data::live::{FuncLive, FuncValLive};
use crate::runtime::vm::manager::{await_call, start_call};
use crate::runtime::vm::shared::{get_func_from_ptr, get_func_vals_from_ptrs, NewValMessage};


mod runtime;
mod binder;

#[derive(Debug, Parser)]
#[command(name = "graphite")]
#[command(about = "CLI tool for using the Graphite VM", long_about = "CLI tool for using the Graphite VM")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug, Parser)]
enum Commands {
    #[command(about = "Runs a program and outputs each result as soon as it is available")]
    #[command(arg_required_else_help = true)]
    Stream {
        #[arg(short = 'i', long = "intermediate")]
        intermediate: String,

        #[arg(short = 'v', long = "verbose", default_value = "false")]
        verbose: bool,

        #[arg(short = 'e', long = "ex-workers")]
        execution_workers: Option<usize>,

        #[arg(short = 'o', long = "or-workers")]
        orchestration_workers: Option<usize>,
    },

    #[command(about = "Runs a program and waits for all results to be available before outputting them")]
    #[command(arg_required_else_help = true)]
    Await {
        #[arg(short = 'i', long = "intermediate")]
        intermediate: String,

        #[arg(short = 'v', long = "verbose", default_value = "false")]
        verbose: bool,

        #[arg(short = 'e', long = "ex-workers")]
        execution_workers: Option<usize>,

        #[arg(short = 'o', long = "or-workers")]
        orchestration_workers: Option<usize>,
    },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Stream { intermediate, verbose, execution_workers, orchestration_workers } => {
            let program: Collection = load_intermediate(&intermediate).unwrap();
            let main_symbol = uuid::Uuid::new_v4().to_string();
            let mmu: Arc<MMU> = Arc::new(MMU::new());
            let mmu2 = mmu.clone();
            let binder = match bind(program, mmu.clone(), Some(main_symbol.clone())) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return;
                },
            };

            let output_callback = Arc::new(move |message: &NewValMessage| {
                log_output(mmu2.clone(), &message.func_val, &message.value);
            });

            let result_callback = Arc::new(move |result: ExecResult<()>| {
                if let Err(e) = result {
                    log_error(format!("Error executing program: {}", e));
                }
            });

            let (ex_count, or_count) = get_worker_counts(execution_workers, orchestration_workers);
            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(ex_count).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(or_count).build().unwrap());

            let main_func: ValueReference = binder.get_path(vec![main_symbol, "main".to_string()]).unwrap();

            start_call(
                mmu.clone(),
                ex_pool,
                or_pool,
                main_func,
                vec![],
                output_callback,
                result_callback,
                verbose
            );

        },

        Commands::Await { intermediate, verbose, execution_workers, orchestration_workers } => {
            let program: Collection = load_intermediate(&intermediate).unwrap();
            let main_symbol = uuid::Uuid::new_v4().to_string();
            let mmu: Arc<MMU> = Arc::new(MMU::new());
            let binder = match bind(program, mmu.clone(), Some(main_symbol.clone())) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return;
                },
            };

            let (ex_count, or_count) = get_worker_counts(execution_workers, orchestration_workers);
            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(ex_count).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(or_count).build().unwrap());

            let main_func: ValueReference = binder.get_path(vec![main_symbol, "main".to_string()]).unwrap();

            let res = await_call(
                mmu.clone(),
                ex_pool,
                or_pool,
                main_func.clone(),
                vec![],
                verbose
            );

            let res: Vec<ValueReference> = match res {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error executing program: {}", e));
                    return;
                },
            };

            let main_func: FuncLive = match get_func_from_ptr(mmu.clone(), &main_func.pointer){
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error getting main function: {}", e));
                    return;
                },
            };

            let output_fn_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(mmu.clone(), &main_func.output_vals) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error getting output function values: {}", e));
                    return;
                },
            };

            for (i, val) in res.iter().enumerate() {
                log_output(mmu.clone(), &output_fn_vals[i], val);
            }
        }
    }

    /// Loads a Graphite JSON Intermediate Language (GJIL) file from the given path and binds to memory.
    fn load_intermediate(path: &str) -> Result<Collection, String> {
        let contents = fs::read_to_string(path).map_err(|e| format!("Error reading intermediate file: {}", e))?;
        let program: Collection = serde_json::from_str(&contents).map_err(|e| format!("Error parsing intermediate JSON: {}", e))?;

        Ok(program)
    }

    fn bind(program: Collection, mmu: Arc<MMU>, program_symbol: Option<Symbol>) -> Result<Binder, String> {
        let program_symbol = program_symbol.unwrap_or_else(|| "main".to_string());
        let mut binder = Binder { mmu, symbol_table: HashMap::new() };
        binder.store_collection(program, program_symbol).map_err(|e| format!("Error binding program: {}", e))?;

        Ok(binder)
    }

    fn get_worker_counts(
        execution_workers: Option<usize>,
        orchestration_workers: Option<usize>,
    ) -> (usize, usize) {
        // if we know one but not the other, use the known value for both
        // if we know neither, use the number of CPUs

        let ex_count = match execution_workers {
            Some(v) => v,
            None => orchestration_workers.unwrap_or_else(|| num_cpus::get()),
        };

        let or_count = match orchestration_workers {
            Some(v) => v,
            None => execution_workers.unwrap_or_else(|| num_cpus::get()),
        };

        (ex_count, or_count)
    }

    fn log_async(message: String) {
        let stdout = io::stdout();
        let _ = writeln!(&mut stdout.lock(),
                         "{}", message
        );
    }

    fn log_error(message: String) {
        let stderr = io::stderr();
        let _ = writeln!(&mut stderr.lock(),
                         "{}", message
        );
    }


    fn log_output(mmu: Arc<MMU>, func_val: &FuncValLive, value: &ValueReference) {
        let symbol = match &func_val.symbol {
            Some(s) => s,
            None => &func_val.guid,
        };

        let stored = match mmu.get_ref_value(value) {
            Ok(v) => v,
            Err(e) => {
                log_error(format!("Error getting output value: {}", e));
                return;
            },
        };
        let val = jsonify(mmu.clone(), &stored);
        log_async(format!("out | {}: {}", symbol, val));
    }

}