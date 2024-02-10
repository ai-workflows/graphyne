use std::sync::Arc;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::value_ref::ValueReference;
use crate::core::data::live::{LiveData};
use crate::core::vm::mmu::mmu::{MMU, store_value, value_ref_from_ptr};
use crate::core::vm::operator::functions::call::handle_call_function;


/// Applies a function to each item in a list, returning a new list of the results.
pub fn map(mmu: Arc<MMU>, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let func = mmu.get_ref_value(func)
        .map_err(|msg| format!("Failed to get function: {}", msg))?
        .as_live().as_func()
        .ok_or_else(|| "Cannot map a non-function value".to_string())??;

    let list_val = mmu.get_ref_value(list)
        .map_err(|msg| format!("Failed to get list: {}", msg))?
        .as_live().as_list()
        .ok_or_else(|| "Cannot map a function with a non-list value as arguments".to_string())??;

    let result_list: Result<Vec<_>, _> = list_val.iter()
        .map(|item_ptr| {
            value_ref_from_ptr(mmu.clone(), item_ptr.clone())
                .and_then(|item_val_ref| handle_call_function(mmu.clone(), &func, &[item_val_ref]))
                .map(|result| result[0].clone())
        })
        .collect();

    result_list.and_then(|vals| {
        let pointers = vals.iter().map(|val_ref| val_ref.pointer.clone()).collect();
        store_value(mmu.clone(), StoredData::ListStored(pointers))
    })
}


/// Wrapper function to handle lifetime issues with calling reduce.
pub fn handle_reduce(mmu: Arc<MMU>, func: &ValueReference, list: &ValueReference, initial: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    // create a new reference to the initial value to avoid lifetime issues
    let initial = value_ref_from_ptr(mmu.clone(), initial.pointer.clone())?;

    reduce(mmu.clone(), func, list, &initial)
}

// applies a combining function to each item in a list, returning a single result.
pub fn reduce<'a>(mmu: Arc<MMU>, func: &ValueReference, list: &ValueReference, initial: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let func = mmu.get_ref_value(func)
        .map_err(|msg| format!("Failed to get function: {}", msg))?
        .as_live().as_func()
        .ok_or_else(|| "Cannot reduce with a non-function value".to_string())??;

    let list_val = mmu.get_ref_value(list)
        .map_err(|msg| format!("Failed to get list: {}", msg))?
        .as_live().as_list()
        .ok_or_else(|| "Cannot reduce a list with a non-list value as arguments".to_string())??;

    let mut last_result = initial.clone();

    for item_ptr in list_val {
        let item_val_ref = value_ref_from_ptr(mmu.clone(), item_ptr.clone())?;
        let result_val = handle_call_function(mmu.clone(), &func, &[last_result.clone(), item_val_ref])?;
        last_result = result_val[0].clone();
    }

    Ok(vec![last_result])
}


// gets the items in a list that match a given condition
pub fn filter(mmu: Arc<MMU>, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let func = mmu.get_ref_value(func)
        .map_err(|msg| format!("Failed to get function: {}", msg))?
        .as_live().as_func()
        .ok_or_else(|| "Cannot filter with a non-function value".to_string())??;

    let list_val = mmu.get_ref_value(list)
        .map_err(|msg| format!("Failed to get list: {}", msg))?
        .as_live().as_list()
        .ok_or_else(|| "Cannot filter a list with a non-list value as arguments".to_string())??;

    let mut result_list: Vec<ValueReference> = Vec::new();

    for item_ptr in list_val {
        let item_val_ref = value_ref_from_ptr(mmu.clone(), item_ptr.clone())?;
        let result_val = handle_call_function(mmu.clone(), &func, &[item_val_ref.clone()])?;
        let result_val_ref = result_val[0].clone();
        let result_val = mmu.get_ref_value(&result_val_ref)?
            .as_live().as_bool()
            .ok_or_else(|| "Cannot filter a list with a non-bool function".to_string())??;

        if result_val {
            result_list.push(item_val_ref);
        }
    }

    let pointers = result_list.iter().map(|val_ref| val_ref.pointer.clone()).collect();
    store_value(mmu.clone(), StoredData::ListStored(pointers))
}

// #[cfg(test)]
// mod tests {
//     use std::collections::HashMap;
//     use crate::api::functions::{FunctionGraph, FunctionOpNode, FunctionValueNode};
//     use crate::api::GraphiteApi;
//     use crate::api::interface::VmInterface;
//     use crate::core::data::functions::OpCode;
//     use crate::core::data::live::{LiveData};
//     use crate::core::data::stored::StoredData::{FloatStored, IntStored};
//     use crate::core::Symbol;
//     use crate::core::vm::mmu::store_op::StoreOp;
//     use crate::core::vm::value_ref::ValueReference;
//     use crate::core::vm::VM;
//
//     #[test]
//     fn test_sub_call<'a>() {
//         let vm = VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
//             let mut api = GraphiteApi { vm: &vm, symbol_table };
//
//             // Define the values for the first function.
//             let values1: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("num1".into()),
//                 FunctionValueNode::var("num2".into()),
//                 FunctionValueNode::var("sum".into()),
//             ];
//
//             // Define the operations for the first function.
//             let ops1: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
//             ];
//
//             // Create the function graph for the first function.
//             let graph1: FunctionGraph = FunctionGraph::new(values1, ops1, vec!["num1".into(), "num2".into()], vec!["sum".into()]);
//
//             // Store the first function
//             api.store_function(graph1, "add".to_string(), None).unwrap();
//
//             // Define the values for the second function which will compute an average.
//             let values2: Vec<FunctionValueNode> = vec![
//                 // the two numbers that will be averaged
//                 FunctionValueNode::var("num1".into()),
//                 FunctionValueNode::var("num2".into()),
//
//                 // the sum function to call and its result
//                 FunctionValueNode::external("add_func".into(), api.get_ptr("add".into()).unwrap()),
//                 FunctionValueNode::var("sum".into()),
//
//                 // the average result
//                 FunctionValueNode::constant("2".into(), FloatStored(2.0)),
//                 FunctionValueNode::var("average".into()),
//             ];
//
//             // Define the operations for the second function.
//             let ops2: Vec<FunctionOpNode> = vec![
//                 // call the add function and divide the result by 2
//                 FunctionOpNode::call("add_func".into(), vec!["num1".into(), "num2".into()], vec!["sum".into()]),
//                 FunctionOpNode::new(OpCode::Div, vec!["sum".into(), "2".into()], "average".into()),
//             ];
//
//             // Create the function graph for the second function.
//             let graph2: FunctionGraph = FunctionGraph::new(values2, ops2, vec!["num1".into(), "num2".into()], vec!["average".into()]);
//
//             // Store the second function
//             api.store_function(graph2, "average".to_string(), None).unwrap();
//
//             // test the second function
//             api.store_int(10, "num1".to_string()).unwrap();
//             api.store_int(20, "num2".to_string()).unwrap();
//
//             api.execute(vec!["average".to_string()], vec!["num1".to_string(), "num2".to_string()], vec!["average".to_string()]).unwrap();
//
//             let result = api.get("average".to_string()).unwrap();
//             assert_eq!(result.as_live().as_float().unwrap(), Ok(15.0));
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_reduce<'a>() {
//         let vm = VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
//             let mut api = GraphiteApi { vm: &vm, symbol_table };
//
//             // Define the add function.
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
//             api.store_function(graph, "add".to_string(), None).unwrap();
//
//             // Define the sum_list function.
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("sum".into()),
//                 FunctionValueNode::external("add_func".into(), api.get_ptr("add".into()).unwrap()),
//                 FunctionValueNode::constant("0".into(), IntStored(0)),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Reduce, vec!["add_func".into(), "list".into(), "0".into()], "sum".into())
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["sum".into()]);
//             api.store_function(graph, "sum_list".to_string(), None).unwrap();
//
//             // Test the sum_list function.
//             let vals = vec![10, 20, 30, 40];
//             let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();
//             api.store_list(val_symbols, "test_list".to_string()).unwrap();
//
//             api.execute(vec!["sum_list".to_string()], vec!["test_list".to_string()], vec!["sum".to_string()]).unwrap();
//             let result = api.get("sum".to_string()).unwrap();
//
//             assert_eq!(result.as_live().as_int().unwrap(), Ok(100));
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_map() {
//         let vm = VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
//             let mut api = GraphiteApi { vm: &vm, symbol_table };
//
//             // Define the double function.
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("num".into()),
//                 FunctionValueNode::var("double".into()),
//                 FunctionValueNode::constant("two".into(), IntStored(2)),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Mul, vec!["num".into(), "two".into()], "double".into())
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num".into()], vec!["double".into()]);
//
//             api.store_function(graph, "double".to_string(), None).unwrap();
//
//             // Define the double_list function.
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("double_list".into()),
//                 FunctionValueNode::external("double_func".into(), api.get_ptr("double".into()).unwrap()),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Map, vec!["double_func".into(), "list".into()], "double_list".into())
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["double_list".into()]);
//
//             api.store_function(graph, "double_list".to_string(), None).unwrap();
//
//             // Test the double_list function.
//             let vals = vec![10, 20, 30, 40];
//             let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();
//
//             api.store_list(val_symbols, "test_list".to_string()).unwrap();
//
//             api.execute(vec!["double_list".to_string()], vec!["test_list".to_string()], vec!["list_doubled".to_string()]).unwrap();
//
//             let result = api.get("list_doubled".to_string()).unwrap();
//             let result_live = result.as_live().as_list().unwrap().unwrap();
//
//             assert_eq!(result_live.len(), 4);
//
//             // define a function to get the values from the list
//             let values = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("index".into()),
//                 FunctionValueNode::var("item".into()),
//             ];
//
//             let ops = vec![
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "index".into()], "item".into())
//             ];
//
//             let graph = FunctionGraph::new(values, ops, vec!["list".into(), "index".into()], vec!["item".into()]);
//             api.store_function(graph, "get_item".to_string(), None).unwrap();
//
//             for i in 0..vals.len() {
//                 api.store_int(i as i64, "index".to_string()).unwrap();
//
//                 api.execute(vec!["get_item".to_string()], vec!["list_doubled".to_string(), "index".into()], vec!["item".to_string()]).unwrap();
//                 let item = api.get("item".to_string()).unwrap();
//                 let item = item.as_live().as_int().unwrap().unwrap();
//
//                 assert_eq!(item, vals[i] * 2);
//
//                 api.drop("index".to_string()).unwrap();
//                 api.drop("item".to_string()).unwrap();
//             }
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_filter<'a>() {
//         let vm = VM::new(2, 2);
//
//         {
//             let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
//             let mut api = GraphiteApi { vm: &vm, symbol_table };
//
//             // Define the is_even function.
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("num".into()),
//                 FunctionValueNode::constant("two".into(), IntStored(2)),
//                 FunctionValueNode::var("remainder".into()),
//                 FunctionValueNode::constant("0".into(), IntStored(0)),
//                 FunctionValueNode::var("is_even".into()),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Mod, vec!["num".into(), "two".into()], "remainder".into()),
//                 FunctionOpNode::new(OpCode::Equal, vec!["remainder".into(), "0".into()], "is_even".into()),
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num".into()], vec!["is_even".into()]);
//             api.store_function(graph, "is_even".to_string(), None).unwrap();
//
//             // Define the filter_list function.
//             let values: Vec<FunctionValueNode> = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("filtered_list".into()),
//                 FunctionValueNode::external("is_even_func".into(), api.get_ptr("is_even".into()).unwrap()),
//             ];
//
//             let ops: Vec<FunctionOpNode> = vec![
//                 FunctionOpNode::new(OpCode::Filter, vec!["is_even_func".into(), "list".into()], "filtered_list".into())
//             ];
//
//             let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["filtered_list".into()]);
//             api.store_function(graph, "filter_list".to_string(), None).unwrap();
//
//             // Test the filter_list function.
//             let vals = vec![5, 10, 15, 20];
//             let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();
//             api.store_list(val_symbols, "test_list".to_string()).unwrap();
//
//             api.execute(vec!["filter_list".to_string()], vec!["test_list".to_string()], vec!["filtered_list".to_string()]).unwrap();
//             let result = api.get("filtered_list".to_string()).unwrap();
//             let result_live = result.as_live().as_list().unwrap().unwrap();
//
//             assert_eq!(result_live.len(), 2);
//
//             // define a function to get the values from the list
//             let values = vec![
//                 FunctionValueNode::var("list".into()),
//                 FunctionValueNode::var("index".into()),
//                 FunctionValueNode::var("item".into()),
//             ];
//
//             let ops = vec![
//                 FunctionOpNode::new(OpCode::Get, vec!["list".into(), "index".into()], "item".into())
//             ];
//
//             let graph = FunctionGraph::new(values, ops, vec!["list".into(), "index".into()], vec!["item".into()]);
//             api.store_function(graph, "get_item".to_string(), None).unwrap();
//
//             for i in 0..result_live.len() {
//                 api.store_int(i as i64, "index".to_string()).unwrap();
//
//                 api.execute(vec!["get_item".to_string()], vec!["filtered_list".to_string(), "index".into()], vec!["item".to_string()]).unwrap();
//                 let item = api.get("item".to_string()).unwrap();
//                 let item = item.as_live().as_int().unwrap().unwrap();
//
//                 match i {
//                     0 => assert_eq!(item, 10),
//                     1 => assert_eq!(item, 20),
//                     _ => panic!("unexpected index")
//                 }
//
//                 api.drop("index".to_string()).unwrap();
//                 api.drop("item".to_string()).unwrap();
//             }
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
// }