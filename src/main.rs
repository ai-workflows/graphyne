use std::collections::HashMap;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fs;
use crate::api::GraphiteApi;
use crate::api::interface::VmInterface;
use crate::api::program::Program;
use crate::core::Symbol;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;


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
            let program: Program= match serde_json::from_str(&contents) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error parsing program JSON: {}", e);
                    return;
                },
            };

            let vm = VM::new(4);
            let symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
            let mut api = GraphiteApi { vm: &vm, symbol_table };

            match api.execute_program(&program) {
                Ok(_) => {},
                Err(e) => {
                    println!("Error executing program: {}", e);
                    return;
                },
            }

            for output in program.outputs {
                match api.get(output.clone()) {
                    Ok(v) => println!("{}: {:}", output, api.jsonify(&v)),
                    Err(e) => println!("Error getting output symbol: {}", e),
                }
            }
        },
    }

}