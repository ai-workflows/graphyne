use clap::{Parser};
use std::sync::Arc;
use crate::api::{await_call_v2, bind, load_intermediate, log_async, log_error, stream_call_v2};
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::live::PointerLive;
use crate::runtime::static_state::state::StaticState;


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

        #[arg(short = 'w', long = "workers")]
        workers: Option<usize>,
    },

    #[command(about = "Runs a program and waits for all results to be available before outputting them")]
    #[command(arg_required_else_help = true)]
    Await {
        #[arg(short = 'i', long = "intermediate")]
        intermediate: String,

        #[arg(short = 'v', long = "verbose", default_value = "false")]
        verbose: bool,

        #[arg(short = 'w', long = "workers")]
        workers: Option<usize>,
    },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Stream { intermediate, verbose, workers } => {
            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return;
                }
            };
            let main_collection_symbol = uuid::Uuid::new_v4().to_string();

            let static_state: Arc<StaticState> = bind(program, Some(main_collection_symbol.clone()))
                .map_err(|e| log_error(format!("Error binding program: {}", e)))
                .unwrap();

            let (num_outputs, outputs_receiver) = stream_call_v2(
                vec![main_collection_symbol, "main".to_string()],
                vec![],
                static_state.clone(),
                workers,
            );

            for _ in 0..num_outputs {
                let res = outputs_receiver.recv().unwrap();
                log_async(format!("out | {}: {}", res.0, jsonify(res.1.as_ref())));
            }
        },

        Commands::Await { intermediate, verbose, workers } => {
            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return;
                }
            };

            let main_collection_symbol = uuid::Uuid::new_v4().to_string();

            let static_state: Arc<StaticState> = bind(program, Some(main_collection_symbol.clone()))
                .map_err(|e| log_error(format!("Error binding program: {}", e)))
                .unwrap();

            let res: Vec<PointerLive> = await_call_v2(
                vec![main_collection_symbol, "main".to_string()],
                vec![],
                static_state.clone(),
                workers,
            );

            for (i, v) in res.iter().enumerate() {
                log_async(format!("out | {}: {}", i, jsonify(v.as_ref())));
            }
        }
    }
}