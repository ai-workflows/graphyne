use std::collections::HashMap;
use serde::Deserialize;
use crate::api::collections::c_const::CollectionConst;
use crate::api::collections::collection::Collection;
use crate::api::GraphiteApi;
use crate::core::{ExecResult, Symbol, SymbolPath};
use crate::core::data::live::FuncLive;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

#[derive(Debug, Clone, Deserialize)]
pub struct Program {
    /// The program's collections.
    pub collections: HashMap<Symbol, Collection>,

    /// The path to the program's entry point.
    pub main: SymbolPath,

    /// Input values to the program passed as arguments.
    pub inputs: Vec<CollectionConst>,

    /// THe symbols of the values that are output by the program.
    pub outputs: Vec<Symbol>,
}

impl<'a> GraphiteApi<'a>  {
    pub fn execute_program(&mut self, program: &Program) -> ExecResult<()> {
        // store the collections
        let collections_vec: Vec<(Collection, Symbol)> = program.collections.iter().map(|(k, v)| (v.clone(), k.clone())).collect();
        match self.store_collections(collections_vec) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }

        // get the main func
        let main_ref = match self.get_path(program.main.clone()) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let main: FuncLive = match self.vm.get_ref_value(&main_ref) {
            Ok(v) => match v {
                StoredData::FuncStored(f) => f,
                _ => return Err("Main function is not a function".to_string()),
            },
            Err(e) => return Err(e),
        };
        drop(main_ref);

        // verify that the number of inputs is correct
        if main.input_vals.len() != program.inputs.len() {
            return Err(format!("Number of inputs does not match number of inputs for function {:?}.", main));
        }

        // store the inputs
        let mut input_refs = Vec::new();
        for input in &program.inputs {
            let input_ref = match self.vm.store_cc_data(input.0.clone()) {
                Ok(v) => v[0].clone(),
                Err(e) => return Err(e),
            };
            input_refs.push(input_ref);
        }

        // call the main func
        let result: Vec<ValueReference> = match self.vm.handle_call_function(&main, &input_refs) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        // verify that the number of outputs is correct
        if result.len() != program.outputs.len() {
            return Err(format!("Expected {} outputs, but got {}", program.outputs.len(), result.len()));
        }

        drop(main);

        // store the outputs
        for (i, output) in program.outputs.iter().enumerate() {
            let output_ref = result[i].clone();
            self.symbol_table.insert(output.clone(), output_ref);
        }

        return Ok(());
    }
}