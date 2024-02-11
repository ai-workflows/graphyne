use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::data::live::{PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult, Symbol, SymbolPath};
use crate::runtime::data::live::live_data::TypeLive;
use crate::runtime::data::stored::StoredData::DictStored;
use crate::runtime::mmu::mmu::{execute_store, MMU, value_ref_from_ptr};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;

/// The binder is responsible for loading GJIL (Graphite JSON Intermediate Language) into memory
/// where it can be executed.
pub struct Binder {
    pub symbol_table: HashMap<Symbol, ValueReference>,
    pub mmu: Arc<MMU>,
}

impl Binder {
    pub fn store_value(&mut self,
                   operation: StoreOp,
                   symbol: Symbol) -> ExecResult<()> {
        let store_result: Vec<ValueReference> = execute_store(self.mmu.clone(), operation).unwrap();
        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    /// Gets the reference to a value at the given symbol path.
    pub fn get_path(&self,
                    path: SymbolPath
    ) -> ExecResult<ValueReference> {
        // represent the symbol table as a stored dict
        let mut context: Option<ValueReference> = None;

        for symbol in path.clone() {
            // get the value of the current context, or convert the symbol table to a dict if this is the first iteration
            let context_value: StoredData = match context {
                Some(context) => match self.mmu.get_ref_value(&context) {
                    Ok(context_stored) => context_stored,
                    Err(err) => return Err(format!("Error getting symbol {} for path {:?}: {}", symbol, path.clone(), err))
                },
                None => {
                    DictStored(self.symbol_table.iter().map(|(symbol, val_ref)| (symbol.clone(), val_ref.pointer.clone())).collect::<HashMap<Symbol, PointerLive>>())
                }
            };

            // get the current context as a dict
            let dict = match context_value {
                DictStored(dict) => dict,
                _ => return Err(format!("Context at symbol {} for path {:?} is not a dict.", symbol, path))
            };

            // get the pointer at the current symbol
            let symbol_ptr = match dict.get(&symbol) {
                Some(symbol_ptr) => symbol_ptr,
                None => return Err(format!("Symbol {} not found for path {:?}.", symbol, path))
            };

            // convert the pointer to a value reference
            context = match value_ref_from_ptr(self.mmu.clone(), symbol_ptr.clone()) {
                Ok(symbol_ref) => Some(symbol_ref),
                Err(err) => return Err(format!("Error getting value reference for symbol {} for path {:?}: {}", symbol, path, err))
            };
        }

        Ok(context.unwrap())
    }
}

// #[cfg(test)]
// mod tests {
//     use std::intermediate::HashMap;
//     use crate::binder::functions::{FunctionGraph, FunctionOpNode, FunctionValueNode};
//     use crate::binder::GraphiteApi;
//     use crate::binder::interface::VmInterface;
//     use crate::runtime::data::functions::OpCode;
//     use crate::runtime::data::live::LiveData;
//     use crate::runtime::{Symbol};
//     use crate::runtime::data::stored::StoredData::IntStored;
//     use crate::runtime::vm::value_ref::ValueReference;
//     use crate::runtime::vm::VM;
//
//     #[test]
//     fn test_api<'a>() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
//
//             let mut binder = GraphiteApi { vm, symbol_table };
//
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("num1".into()),
//                 FunctionValueNode::var("num2".into()),
//                 FunctionValueNode::var("sum".into()),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);
//
//             binder.store_function(graph, "add".to_string(), None).unwrap();
//
//             binder.store_int(5, "num1".to_string()).unwrap();
//             binder.store_float(10.0, "num2".to_string()).unwrap();
//
//             binder.execute(vec!["add".to_string()], vec!["num1".to_string(), "num2".to_string()], vec!["sum".to_string()]).unwrap();
//
//             let result = binder.get("sum".to_string()).unwrap();
//             assert_eq!(result.as_live().as_int().unwrap(), Ok(15));
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_calculate_statistics<'a>() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
//             let mut binder = GraphiteApi { vm, symbol_table };
//
//             // Define the values (variables).
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("sum1".into()),
//                 FunctionValueNode::var("sum2".into()),
//                 FunctionValueNode::var("sum_final".into()),
//                 FunctionValueNode::var("average".into()),
//                 FunctionValueNode::var("is_large_sum".into()),
//                 FunctionValueNode::var("sum_as_float".into()),
//                 FunctionValueNode::var("list_length".into()),
//                 FunctionValueNode::constant("100".into(), IntStored(100)),
//                 FunctionValueNode::constant("0".into(), IntStored(0)),
//                 FunctionValueNode::constant("1".into(), IntStored(1)),
//                 FunctionValueNode::constant("2".into(), IntStored(2)),
//                 FunctionValueNode::constant("3".into(), IntStored(3)),
//                 FunctionValueNode::var("item1".into()),
//                 FunctionValueNode::var("item2".into()),
//                 FunctionValueNode::var("item3".into()),
//                 FunctionValueNode::var("item4".into()),
//             ];
//
//             // Define the operations.
//             let ops: Vec<FunctionOpNode> = vec![
//                 // get the items from the list
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "0".into()], "item1".into()),
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "1".into()], "item2".into()),
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "2".into()], "item3".into()),
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "3".into()], "item4".into()),
//
//                 // compute the sum
//                 FunctionOpNode::new(OpCode::Add, vec!["item1".into(), "item2".into()], "sum1".into()),
//                 FunctionOpNode::new(OpCode::Add, vec!["item3".into(), "item4".into()], "sum2".into()),
//                 FunctionOpNode::new(OpCode::Add, vec!["sum1".into(), "sum2".into()], "sum_final".into()),
//
//                 // compute the average
//                 FunctionOpNode::new(OpCode::Length, vec!["list".into()], "list_length".into()),
//                 FunctionOpNode::new(OpCode::Div, vec!["sum_final".into(), "list_length".into()], "average".into()),
//
//                 // compute whether the sum is large
//                 FunctionOpNode::new(OpCode::GreaterThan, vec!["sum_final".into(), "100".into()], "is_large_sum".into()),
//
//                 // get the sum as a float
//                 FunctionOpNode::new(OpCode::AsFloat, vec!["sum_final".into()], "sum_as_float".into()),
//             ];
//
//             // Create the function graph.
//             let graph: FunctionGraph = FunctionGraph::new(
//                 values,
//                 ops,
//                 vec!["list".into()], // Initial Inputs
//                 vec!["sum_final".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()], // Returned Outputs
//             );
//
//             // Store the function.
//             binder.store_function(graph, "calculate_statistics".to_string(), None).unwrap();
//
//             // Test the function with a sample list.
//             binder.store_int(10, "num1".to_string()).unwrap();
//             binder.store_int(20, "num2".to_string()).unwrap();
//             binder.store_int(30, "num3".to_string()).unwrap();
//             binder.store_int(40, "num4".to_string()).unwrap();
//
//             binder.store_list(vec!["num1".into(), "num2".into(), "num3".into(), "num4".into()], "list".to_string()).unwrap();
//             binder.execute(vec!["calculate_statistics".to_string()], vec!["list".to_string()], vec!["sum".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()]).unwrap();
//
//             // Retrieve and assert the results.
//             let sum_result = binder.get("sum".to_string()).unwrap();
//             let average_result = binder.get("average".to_string()).unwrap();
//             let is_large_sum_result = binder.get("is_large_sum".to_string()).unwrap();
//             let sum_as_float_result = binder.get("sum_as_float".to_string()).unwrap();
//
//             assert_eq!(sum_result.as_live().as_int().unwrap(), Ok(100));
//             assert_eq!(average_result.as_live().as_float().unwrap(), Ok(25.0));
//             assert_eq!(is_large_sum_result.as_live().as_bool().unwrap(), Ok(false));
//             assert_eq!(sum_as_float_result.as_live().as_float().unwrap(), Ok(100.0));
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
// }