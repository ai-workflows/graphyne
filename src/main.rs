use clap::Parser;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use crate::api::{bind, get_worker_counts, load_intermediate, log_async, log_error, log_info, try_await_call, try_stream_call};
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::live::PointerLive;
use crate::runtime::static_state::state::StaticState;

mod runtime;
mod binder;
mod api;

#[derive(Debug, Parser)]
#[command(name = "graphyne")]
#[command(about = "CLI tool for using the Graphyne VM", long_about = "CLI tool for using the Graphyne VM")]
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

fn log_verbose(enabled: bool, message: impl Into<String>) {
    if enabled {
        log_info(format!("info: {}", message.into()));
    }
}

fn main() -> ExitCode {
    let args = Cli::parse();

    match args.command {
        Commands::Stream { intermediate, verbose, workers } => {
            let worker_count = get_worker_counts(workers);
            log_verbose(verbose, format!("mode=stream intermediate={} workers={}", intermediate, worker_count));

            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return ExitCode::FAILURE;
                }
            };
            let main_collection_symbol = uuid::Uuid::new_v4().to_string();

            let static_state: Arc<StaticState> = match bind(program, Some(main_collection_symbol.clone())) {
                Ok(state) => state,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return ExitCode::FAILURE;
                }
            };

            let (num_outputs, outputs_receiver, error_receiver) = match try_stream_call(
                vec![main_collection_symbol, "main".to_string()],
                vec![],
                static_state.clone(),
                Some(worker_count),
            ) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error starting program: {}", e));
                    return ExitCode::FAILURE;
                }
            };

            log_verbose(verbose, format!("waiting for {} outputs", num_outputs));

            let mut received_outputs = 0usize;
            while received_outputs < num_outputs {
                if let Ok(err) = error_receiver.try_recv() {
                    log_error(format!("Runtime error: {}", err));
                    return ExitCode::FAILURE;
                }

                match outputs_receiver.recv_timeout(std::time::Duration::from_millis(10)) {
                    Ok(res) => {
                        log_async(format!("out | {}: {}", res.0, jsonify(res.1.as_ref())));
                        received_outputs += 1;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        if let Ok(err) = error_receiver.try_recv() {
                            log_error(format!("Runtime error: {}", err));
                            return ExitCode::FAILURE;
                        }
                        return ExitCode::FAILURE;
                    }
                }
            }
        },

        Commands::Await { intermediate, verbose, workers } => {
            let worker_count = get_worker_counts(workers);
            log_verbose(verbose, format!("mode=await intermediate={} workers={}", intermediate, worker_count));

            let program: Collection = match load_intermediate(&intermediate) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error loading intermediate program: {}", e));
                    return ExitCode::FAILURE;
                }
            };

            let main_collection_symbol = uuid::Uuid::new_v4().to_string();

            let static_state: Arc<StaticState> = match bind(program, Some(main_collection_symbol.clone())) {
                Ok(state) => state,
                Err(e) => {
                    log_error(format!("Error binding program: {}", e));
                    return ExitCode::FAILURE;
                }
            };

            let res: Vec<PointerLive> = match try_await_call(
                vec![main_collection_symbol, "main".to_string()],
                vec![],
                static_state.clone(),
                Some(worker_count),
            ) {
                Ok(v) => v,
                Err(e) => {
                    log_error(format!("Error starting program: {}", e));
                    return ExitCode::FAILURE;
                }
            };

            log_verbose(verbose, format!("received {} outputs", res.len()));

            for (i, v) in res.iter().enumerate() {
                log_async(format!("out | {}: {}", i, jsonify(v.as_ref())));
            }
        }
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::{CommandFactory, Parser};

    #[test]
    fn help_uses_graphyne_binary_name() {
        let mut help = Vec::new();
        Cli::command().write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        assert!(help.contains("graphyne"));
        assert!(!help.contains("graphite"));
    }

    #[test]
    fn parses_verbose_flag_for_stream() {
        let cli = Cli::parse_from(["graphyne", "stream", "-i", "program.json", "--verbose"]);

        match cli.command {
            Commands::Stream { intermediate, verbose, workers } => {
                assert_eq!(intermediate, "program.json");
                assert!(verbose);
                assert_eq!(workers, None);
            }
            _ => panic!("expected stream command"),
        }
    }

    #[test]
    fn parses_verbose_flag_for_await() {
        let cli = Cli::parse_from(["graphyne", "await", "-i", "program.json", "-v", "-w", "3"]);

        match cli.command {
            Commands::Await { intermediate, verbose, workers } => {
                assert_eq!(intermediate, "program.json");
                assert!(verbose);
                assert_eq!(workers, Some(3));
            }
            _ => panic!("expected await command"),
        }
    }
}
