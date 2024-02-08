use std::collections::HashMap;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fs;
use std::sync::Arc;
use crate::api::collections::collection::Collection;
use crate::api::GraphiteApi;
use crate::api::interface::VmInterface;
// use crate::api::program::Program;
use crate::core::Symbol;
use crate::core::vm::mmu::mmu::MMU;
use crate::core::vm::value_ref::ValueReference;


mod core;
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
    #[command(about = "Runs a program from a json file")]
    #[command(arg_required_else_help = true)]
    Run {
        path: String
    },

    // #[command(about = "Executes a specified function with inputs and outputs")]
    // #[command(arg_required_else_help = true)]
    // Execute {
    //     function: String
    // }
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Run { path } => {
            let contents = fs::read_to_string(path).expect("Something went wrong reading the file");
            let program: Collection = match serde_json::from_str(&contents) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error parsing program JSON: {}", e);
                    return;
                },
            };

            let mmu: MMU = MMU::new();

            // let vm = VM::new(2, 2);
            let symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
            let mut api = GraphiteApi { mmu: Arc::new(mmu), symbol_table };

            let results = match api.execute_program(&program) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error executing program: {}", e);
                    return;
                },
            };

            for result in results {
                let guid = result.0;
                let symbol: Option<Symbol> = result.1;

                match api.get(guid.clone()) {
                    Ok(v) => match symbol {
                        Some(s) => println!("{}: {}", s, api.jsonify(&v)),
                        None => println!("{}: {}", guid, api.jsonify(&v)),
                    },
                    Err(e) => println!("Error getting output symbol: {}", e),
                }
            }

            // for output in program.outputs {
            //     match api.get(output.clone()) {
            //         Ok(v) => println!("{}: {:}", output, api.jsonify(&v)),
            //         Err(e) => println!("Error getting output symbol: {}", e),
            //     }
            // }
        },
    }

}