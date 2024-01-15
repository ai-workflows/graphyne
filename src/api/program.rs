use crate::api::collections::collection::Collection;
use crate::api::GraphiteApi;
use crate::core::{ExecResult, Symbol, SymbolPath};
use crate::core::data::live::FuncLive;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

// #[derive(Debug, Clone, Deserialize)]
// pub struct Program {
//     /// The program's collections.
//     pub collections: HashMap<Symbol, Collection>,
//
//     /// The path to the program's entry point.
//     pub main: SymbolPath,
//
//     /// Input values to the program passed as arguments.
//     pub inputs: Vec<CollectionConst>,
//
//     /// THe symbols of the values that are output by the program.
//     pub outputs: Vec<Symbol>,
// }

impl<'a> GraphiteApi<'a>  {
    pub fn execute_program(&mut self, program: &Collection) -> ExecResult<Vec<(Symbol, Option<Symbol>)>> {
        // store the main collection
        match self.store_collection(program.clone(), "main".to_string()) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }

        // get the main func
        let main_path: SymbolPath = vec!["main".to_string(), "main".to_string()];
        let main_ref = match self.get_path(main_path) {
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

        // call the main func
        let result: Vec<ValueReference> = match self.vm.handle_call_function(&main, &vec![]) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        // verify that the number of outputs is correct
        if result.len() != main.output_vals.len() {
            return Err(format!("Expected {} outputs, but got {}", main.output_vals.len(), result.len()));
        }

        // store the outputs
        let mut results: Vec<(Symbol, Option<Symbol>)> = vec![];

        for (i, output_val_ptr) in main.output_vals.iter().enumerate() {
            let output_val_ref = self.vm.value_ref_from_ptr(output_val_ptr.clone()).unwrap();
            let output_val = match self.vm.get_ref_value(&output_val_ref) {
                Ok(v) => match v {
                    StoredData::FuncValStored(f) => f,
                    _ => return Err("Output value is not a function value".to_string()),
                }
                Err(e) => return Err(e),
            };

            let output_ref = result[i].clone();
            self.symbol_table.insert(output_val.guid.clone(), output_ref);

            results.push((output_val.guid.clone(), output_val.symbol.clone()));
        }

        return Ok(results);
    }
}