use clap::{Parser};
use std::sync::Arc;
use std::thread;
use crate::api::{await_call, bind, load_intermediate, log_async, log_error, stream_call};
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;


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
            let program: Collection = load_intermediate(&intermediate).unwrap();
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

            let (outputs_sender, outputs_receiver) = std::sync::mpsc::channel();

            let mmu2 = mmu.clone();
            thread::spawn(move || {
                for (k, v) in outputs_receiver {
                    log_async(format!("out | {}: {}", k, jsonify(mmu2.clone(), &v)));
                }
            });

            let result = stream_call(
                main_func,
                vec![],
                mmu.clone(),
                outputs_sender.clone(),
                verbose,
                execution_workers,
                orchestration_workers,
            );

            match result {
                Ok(_) => log_async("result | success".to_string()),
                Err(e) => log_error(format!("result | error: {}", e))
            }
        },

        Commands::Await { intermediate, verbose, execution_workers, orchestration_workers } => {
            let program: Collection = load_intermediate(&intermediate).unwrap();
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
                    panic!("Error executing call: {}", e);
                }
            };

            for (k, v) in res {
                println!("out | {}: {}", k, jsonify(mmu.clone(), &v));
            }
        }
    }



}