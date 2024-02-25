use clap::{Parser};
use std::sync::Arc;
use crate::api::{await_call, bind, load_intermediate, log_async, log_error, stream_call};
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::manager::StreamResult;


mod runtime;
mod binder;
mod api;

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
            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return;
                }
            };
            let main_collection_symbol = uuid::Uuid::new_v4().to_string();
            let mmu: Arc<MMU> = Arc::new(MMU::new());
            let binder = match bind(program, mmu.clone(), Some(main_collection_symbol.clone())) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return;
                },
            };

            let main_func: ValueReference = binder.get_path(vec![main_collection_symbol, "main".to_string()]).unwrap();

            let (outputs_sender, outputs_receiver) = std::sync::mpsc::channel::<StreamResult>();

            let start_res = stream_call(
                main_func,
                vec![],
                mmu.clone(),
                outputs_sender.clone(),
                verbose,
                execution_workers,
                orchestration_workers,
            );

            let num_expected_outputs = match start_res {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error starting program: {}", e));
                    return;
                }
            };

            for _ in 0..num_expected_outputs {
                let res = outputs_receiver.recv().unwrap();
                match res {
                    StreamResult::Output(fn_val, val_ref) => {
                        log_async(format!("out | {}: {}", fn_val.symbol.unwrap_or(fn_val.guid), jsonify(mmu.clone(), &mmu.get_ref_value(&val_ref).unwrap())));
                    },
                    StreamResult::Error(e) => {
                        log_error(format!("result | error: {}", e));
                    }
                }
            }
        },

        Commands::Await { intermediate, verbose, execution_workers, orchestration_workers } => {
            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return;
                }
            };

            let main_collection_symbol = uuid::Uuid::new_v4().to_string();
            let mmu: Arc<MMU> = Arc::new(MMU::new());
            let binder = match bind(program, mmu.clone(), Some(main_collection_symbol.clone())) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return;
                },
            };

            let main_func: ValueReference = binder.get_path(vec![main_collection_symbol, "main".to_string()]).unwrap();

            let res = await_call(
                main_func,
                vec![],
                mmu.clone(),
                verbose,
                execution_workers,
                orchestration_workers
            );

            let res = match res {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("result | error: {}", e));
                    return;
                }
            };

            log_async("result | success".to_string());

            for (k, v) in res {
                let stored = mmu.get_ref_value(&v).unwrap();
                log_async(format!("out | {}: {}", k, jsonify(mmu.clone(), &stored)));
            }
        }
    }
}