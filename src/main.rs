use clap::{Parser};
use std::sync::Arc;
use std::thread;
use crate::api::{bind, call, load_intermediate, log_async, log_error};
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
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

            let mmu2 = mmu.clone();
            thread::spawn(move || {
                let _ = call(
                    main_func,
                    vec![],
                    mmu2,
                    verbose,
                    workers,
                    Some(outputs_sender.clone())
                );
            });

            let mut output_count = 0;
            let mut expected_output_count: Option<usize> = None;

            loop {
                let res = outputs_receiver.recv().unwrap();
                match res {
                    StreamResult::NumOutputs(num) => {
                        expected_output_count = Some(num);
                    },
                    StreamResult::Output(fn_val, val_ref) => {
                        log_async(format!("out | {}: {}", fn_val.symbol.unwrap_or(fn_val.guid), jsonify(mmu.clone(), &mmu.get_ref_value(&val_ref).unwrap())));
                        output_count += 1;
                        if let Some(expected) = expected_output_count {
                            if output_count >= expected {
                                break;
                            }
                        }
                    },
                    StreamResult::Error(e) => {
                        log_error(format!("result | error: {}", e));
                        return;
                    }
                }
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
            let mmu: Arc<MMU> = Arc::new(MMU::new());
            let binder = match bind(program, mmu.clone(), Some(main_collection_symbol.clone())) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return;
                },
            };

            let main_func: ValueReference = binder.get_path(vec![main_collection_symbol, "main".to_string()]).unwrap();

            let res: ExecResult<Vec<ValueReference>> = call(
                main_func.clone(),
                vec![],
                mmu.clone(),
                verbose,
                workers,
                None
            );

            let res: Vec<ValueReference> = match res {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("result | error: {}", e));
                    return;
                }
            };

            log_async("result | success".to_string());

            let main_func_stored = mmu.get_ref_value(&main_func).unwrap();
            let main_func = match main_func_stored.as_ref() {
                StoredData::FuncStored(f) => f,
                _ => panic!("main function is not a function")
            };

            for i in 0..res.len() {
                let output_fn_val_ptr = main_func.output_vals.get(i).unwrap();
                let output_fn_val = mmu.get_ptr_value(output_fn_val_ptr).unwrap();
                match output_fn_val.as_ref() {
                    StoredData::FuncValStored(k) => {
                        let k_symbol = k.symbol.clone().unwrap_or(k.guid.clone());
                        log_async(format!("out | {}: {}", k_symbol, jsonify(mmu.clone(), &mmu.get_ref_value(&res[i]).unwrap())));
                    },
                    _ => panic!("output value is not a function value")
                }
            }
        }
    }
}