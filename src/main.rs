mod core;
mod api;

fn main() {
    println!("Hello, world!");
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use maplit::hashmap;
    use crate::api::{GraphiteApi};
    use crate::api::collections::c_const::{CCData};
    use crate::api::collections::collection::Collection;
    use crate::api::collections::func::{CFnValueNode, CollectionFunc, CollectionFuncGraph};
    use crate::api::functions::{FunctionGraph, FunctionOpNode, FunctionValueNode};
    use crate::api::interface::VmInterface;
    use crate::core::data::functions::{OpCode};
    use crate::core::data::live::{LiveData, StringLive};
    use crate::core::data::stored::StoredData::{FloatStored, IntStored};
    use crate::core::Symbol;
    use crate::core::vm::ops::Operation;
    use crate::core::vm::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    fn test_dict(vm: &mut VM, dict: HashMap<StringLive, StringLive>) {
        vm.reset();

        let mut new_dict: HashMap<StringLive, &ValueReference> = HashMap::new();

        let results: HashMap<StringLive, Vec<ValueReference>> = dict.iter().map(|(k, v)| {
            let v_ptr = vm.execute_store(StoreOp::StoreString(v.clone())).unwrap();
            (k.clone(), v_ptr)
        }).collect();

        for (k, v) in results.iter() {
            new_dict.insert(k.clone(), v.get(0).unwrap());
        }

        // there should be len(dict) objects in the VM
        assert_eq!(vm.object_count(), dict.len());

        let st_d_result = vm.execute_store(StoreOp::StoreDict(new_dict.clone())).unwrap();
        let dict_ptr = st_d_result.get(0).unwrap();

        // there should be len(dict) + 1 objects in the VM
        assert_eq!(vm.object_count(), dict.len() + 1);

        // test dict length
        let len_op = Operation::Length(dict_ptr);
        let len_result = vm.execute_op(len_op).unwrap();
        let len_ref = len_result.get(0).unwrap();
        let len = vm.get_ref_value(len_ref).unwrap().as_live().as_int().unwrap().ok().unwrap();

        assert_eq!(len as usize, dict.len());

        // test dict get
        for (k, v) in dict.iter() {
            let st_key_result = vm.execute_store(StoreOp::StoreString(k.clone())).unwrap();

            let get_op = Operation::GetItem(dict_ptr, st_key_result.get(0).unwrap());
            let get_result = vm.execute_op(get_op).unwrap();
            let item_ref = get_result.get(0).unwrap();
            let item = vm.get_ref_value(item_ref).unwrap();
            let item = item.as_live().as_string().unwrap().ok().unwrap();

            assert_eq!(item, v.clone());
        }

        // test dict set
        let st_key_result = vm.execute_store(StoreOp::StoreString("Hello".to_string())).unwrap();
        let new_value_result = vm.execute_store(StoreOp::StoreString("Hello World".to_string())).unwrap();

        let key_ref = st_key_result.get(0).unwrap();
        let new_value_ref = new_value_result.get(0).unwrap();

        let set_op = Operation::SetItem(dict_ptr, key_ref, new_value_ref);
        let set_result = vm.execute_op(set_op).unwrap();
        let new_dict_ref = set_result.get(0).unwrap();

        let get_result = vm.execute_op(Operation::GetItem(new_dict_ref, key_ref)).unwrap();
        let item_ref = get_result.get(0).unwrap();
        let item = vm.get_ref_value(item_ref).unwrap();
        let item = item.as_live().as_string().unwrap().ok().unwrap();

        assert_eq!(item, "Hello World".to_string());
    }



    fn test_func_build(vm: &mut VM) {
        vm.reset();

        let st_return_val_result = vm.execute_store(StoreOp::StoreFunctionVal(Vec::new(), None, false)).unwrap();
        let return_val_ref = st_return_val_result.get(0).unwrap();

        let st_add_buffer_result = vm.execute_store(StoreOp::CreateBuffer).unwrap();
        let add_op_ref = st_add_buffer_result.get(0).unwrap();

        let st_arg1_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false)).unwrap();
        let arg1_ref = st_arg1_result.get(0).unwrap();

        let st_arg2_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false)).unwrap();
        let arg2_ref = st_arg2_result.get(0).unwrap();

        // fill the add buffer with the add op
        let add_op = StoreOp::StoreFunctionOp(OpCode::Add, vec![arg1_ref, arg2_ref], vec![return_val_ref]);
        let fill_add_buffer = Operation::SetBuffer(add_op_ref, add_op.get_stored_data().unwrap());
        vm.execute_op(fill_add_buffer).unwrap();

        // create the function
        let store_func_op = StoreOp::StoreFunction(vec![arg1_ref, arg2_ref], vec![return_val_ref], vec![]);
        let store_func_result = vm.execute_store(store_func_op).unwrap();
        let _func_ref = store_func_result.get(0).unwrap();

        // println!("state: {:#?}", vm.state);

        // test calling the func op
        let context: Arc<RwLock<HashMap<Symbol, ValueReference>>> = Arc::new(RwLock::new(HashMap::new()));
        let arg1_guid = vm.get_ref_value(arg1_ref).unwrap().as_live().as_func_val().unwrap().ok().unwrap().guid;
        let arg2_guid = vm.get_ref_value(arg2_ref).unwrap().as_live().as_func_val().unwrap().ok().unwrap().guid;
        let st_arg1_result = vm.execute_store(StoreOp::StoreInt(5)).unwrap();
        let st_arg2_result = vm.execute_store(StoreOp::StoreInt(10)).unwrap();
        context.write().unwrap().insert(arg1_guid, st_arg1_result.get(0).unwrap().clone());
        context.write().unwrap().insert(arg2_guid, st_arg2_result.get(0).unwrap().clone());

        let add_op_val = vm.get_ref_value(add_op_ref).unwrap().as_live().as_func_op().unwrap().ok().unwrap();
        let call_func_op_rst = vm.handle_call_function_op(&add_op_val, context);
        println!("call_func_op_rst: {:#?}", call_func_op_rst);
        let call_func_op_rst = call_func_op_rst.unwrap();
        assert_eq!(call_func_op_rst.len(), 1);
        let call_func_op_rst = call_func_op_rst.get(0).unwrap();
        let call_func_op_rst = vm.get_ref_value(call_func_op_rst).unwrap().as_live().as_int().unwrap().ok().unwrap();
        assert_eq!(call_func_op_rst, 15);
    }

    fn test_add_func(vm: &mut VM) {
        vm.reset();

        let st_return_val_result = vm.execute_store(StoreOp::StoreFunctionVal(Vec::new(), None, false)).unwrap();
        let return_val_ref = st_return_val_result.get(0).unwrap();

        let st_add_buffer_result = vm.execute_store(StoreOp::CreateBuffer).unwrap();
        let add_op_ref = st_add_buffer_result.get(0).unwrap();

        let st_arg1_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false)).unwrap();
        let arg1_ref = st_arg1_result.get(0).unwrap();

        let st_arg2_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false)).unwrap();
        let arg2_ref = st_arg2_result.get(0).unwrap();

        // fill the add buffer with the add op
        let add_op = StoreOp::StoreFunctionOp(OpCode::Add, vec![arg1_ref, arg2_ref], vec![return_val_ref]);
        let fill_add_buffer = Operation::SetBuffer(add_op_ref, add_op.get_stored_data().unwrap());
        vm.execute_op(fill_add_buffer).unwrap();

        // create the function
        let store_func_op = StoreOp::StoreFunction(vec![arg1_ref, arg2_ref], vec![return_val_ref], vec![]);
        let store_func_result = vm.execute_store(store_func_op).unwrap();
        let func_ref = store_func_result.get(0).unwrap();

        let st_arg1_result = vm.execute_store(StoreOp::StoreInt(5)).unwrap();
        let st_arg2_result = vm.execute_store(StoreOp::StoreInt(10)).unwrap();

        let args: Vec<ValueReference> = vec![st_arg1_result.get(0).unwrap().clone(), st_arg2_result.get(0).unwrap().clone()];

        let func_val = vm.get_ref_value(func_ref).unwrap().as_live().as_func().unwrap().ok().unwrap();

        let call_result = vm.handle_call_function(&func_val, &args).unwrap();
        let call_result = vm.get_ref_value(call_result.get(0).unwrap()).unwrap().as_live().as_int().unwrap().ok().unwrap();

        assert_eq!(call_result, 15);
    }

    fn test_load_fn(vm: &mut VM) {
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num1".into()),
            FunctionValueNode::var("num2".into()),
            FunctionValueNode::var("sum".into()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);


        let load_result = vm.store_function(&graph, None).unwrap();
        let fn_ref = load_result.get(0).unwrap().clone();

        let fn_val = vm.get_ref_value(&fn_ref).unwrap().as_live().as_func().unwrap().ok().unwrap();

        let st_arg1_result = vm.execute_store(StoreOp::StoreInt(5)).unwrap();
        let st_arg2_result = vm.execute_store(StoreOp::StoreInt(10)).unwrap();

        let args: Vec<ValueReference> = vec![st_arg1_result.get(0).unwrap().clone(), st_arg2_result.get(0).unwrap().clone()];

        let call_result = vm.handle_call_function(&fn_val, &args).unwrap();
        let call_result = vm.get_ref_value(call_result.get(0).unwrap()).unwrap().as_live().as_int().unwrap().ok().unwrap();

        assert_eq!(call_result, 15);
    }

    fn test_api<'a>(vm: &'a mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();

        let mut api = GraphiteApi { vm, symbol_table };

        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num1".into()),
            FunctionValueNode::var("num2".into()),
            FunctionValueNode::var("sum".into()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);

        api.store_function(graph, "add".to_string(), None).unwrap();

        api.store_int(5, "num1".to_string()).unwrap();
        api.store_float(10.0, "num2".to_string()).unwrap();

        api.execute(vec!["add".to_string()], vec!["num1".to_string(), "num2".to_string()], vec!["sum".to_string()]).unwrap();

        let result = api.get("sum".to_string()).unwrap();
        assert_eq!(result.as_live().as_int().unwrap(), Ok(15));
    }

    fn test_calculate_statistics<'a>(vm: &'a mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
        let mut api = GraphiteApi { vm, symbol_table };

        // Define the values (variables).
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("sum1".into()),
            FunctionValueNode::var("sum2".into()),
            FunctionValueNode::var("sum_final".into()),
            FunctionValueNode::var("average".into()),
            FunctionValueNode::var("is_large_sum".into()),
            FunctionValueNode::var("sum_as_float".into()),
            FunctionValueNode::var("list_length".into()),
            FunctionValueNode::constant("100".into(), IntStored(100)),
            FunctionValueNode::constant("0".into(), IntStored(0)),
            FunctionValueNode::constant("1".into(), IntStored(1)),
            FunctionValueNode::constant("2".into(), IntStored(2)),
            FunctionValueNode::constant("3".into(), IntStored(3)),
            FunctionValueNode::var("item1".into()),
            FunctionValueNode::var("item2".into()),
            FunctionValueNode::var("item3".into()),
            FunctionValueNode::var("item4".into()),
        ];

        // Define the operations.
        let ops: Vec<FunctionOpNode> = vec![
            // get the items from the list
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "0".into()], "item1".into()),
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "1".into()], "item2".into()),
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "2".into()], "item3".into()),
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "3".into()], "item4".into()),

            // compute the sum
            FunctionOpNode::new(OpCode::Add, vec!["item1".into(), "item2".into()], "sum1".into()),
            FunctionOpNode::new(OpCode::Add, vec!["item3".into(), "item4".into()], "sum2".into()),
            FunctionOpNode::new(OpCode::Add, vec!["sum1".into(), "sum2".into()], "sum_final".into()),

            // compute the average
            FunctionOpNode::new(OpCode::Length, vec!["list".into()], "list_length".into()),
            FunctionOpNode::new(OpCode::Div, vec!["sum_final".into(), "list_length".into()], "average".into()),

            // compute whether the sum is large
            FunctionOpNode::new(OpCode::GreaterThan, vec!["sum_final".into(), "100".into()], "is_large_sum".into()),

            // get the sum as a float
            FunctionOpNode::new(OpCode::AsFloat, vec!["sum_final".into()], "sum_as_float".into()),
        ];

        // Create the function graph.
        let graph: FunctionGraph = FunctionGraph::new(
            values,
            ops,
            vec!["list".into()], // Initial Inputs
            vec!["sum_final".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()] // Returned Outputs
        );

        // Store the function.
        api.store_function(graph, "calculate_statistics".to_string(), None).unwrap();

        // Test the function with a sample list.
        api.store_int(10, "num1".to_string()).unwrap();
        api.store_int(20, "num2".to_string()).unwrap();
        api.store_int(30, "num3".to_string()).unwrap();
        api.store_int(40, "num4".to_string()).unwrap();

        api.store_list(vec!["num1".into(), "num2".into(), "num3".into(), "num4".into()], "list".to_string()).unwrap();
        api.execute(vec!["calculate_statistics".to_string()], vec!["list".to_string()], vec!["sum".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()]).unwrap();

        // Retrieve and assert the results.
        let sum_result = api.get("sum".to_string()).unwrap();
        let average_result = api.get("average".to_string()).unwrap();
        let is_large_sum_result = api.get("is_large_sum".to_string()).unwrap();
        let sum_as_float_result = api.get("sum_as_float".to_string()).unwrap();

        assert_eq!(sum_result.as_live().as_int().unwrap(), Ok(100));
        assert_eq!(average_result.as_live().as_float().unwrap(), Ok(25.0));
        assert_eq!(is_large_sum_result.as_live().as_bool().unwrap(), Ok(false));
        assert_eq!(sum_as_float_result.as_live().as_float().unwrap(), Ok(100.0));
    }

    fn test_sub_call<'a>(vm: &'a mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
        let mut api = GraphiteApi { vm, symbol_table };

        // Define the values for the first function.
        let values1: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num1".into()),
            FunctionValueNode::var("num2".into()),
            FunctionValueNode::var("sum".into()),
        ];

        // Define the operations for the first function.
        let ops1: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
        ];

        // Create the function graph for the first function.
        let graph1: FunctionGraph = FunctionGraph::new(values1, ops1, vec!["num1".into(), "num2".into()], vec!["sum".into()]);

        // Store the first function
        api.store_function(graph1, "add".to_string(), None).unwrap();

        // Define the values for the second function which will compute an average.
        let values2: Vec<FunctionValueNode> = vec![
            // the two numbers that will be averaged
            FunctionValueNode::var("num1".into()),
            FunctionValueNode::var("num2".into()),

            // the sum function to call and its result
            FunctionValueNode::external("add_func".into(), api.get_ptr("add".into()).unwrap()),
            FunctionValueNode::var("sum".into()),

            // the average result
            FunctionValueNode::constant("2".into(), FloatStored(2.0)),
            FunctionValueNode::var("average".into()),
        ];

        // Define the operations for the second function.
        let ops2: Vec<FunctionOpNode> = vec![
            // call the add function and divide the result by 2
            FunctionOpNode::call("add_func".into(), vec!["num1".into(), "num2".into()], vec!["sum".into()]),
            FunctionOpNode::new(OpCode::Div, vec!["sum".into(), "2".into()], "average".into()),
        ];

        // Create the function graph for the second function.
        let graph2: FunctionGraph = FunctionGraph::new(values2, ops2, vec!["num1".into(), "num2".into()], vec!["average".into()]);

        // Store the second function
        api.store_function(graph2, "average".to_string(), None).unwrap();

        // test the second function
        api.store_int(10, "num1".to_string()).unwrap();
        api.store_int(20, "num2".to_string()).unwrap();

        api.execute(vec!["average".to_string()], vec!["num1".to_string(), "num2".to_string()], vec!["average".to_string()]).unwrap();

        let result = api.get("average".to_string()).unwrap();
        assert_eq!(result.as_live().as_float().unwrap(), Ok(15.0));
    }

    fn test_reduce<'a>(vm: &'a mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
        let mut api = GraphiteApi { vm, symbol_table };

        // Define the add function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num1".into()),
            FunctionValueNode::var("num2".into()),
            FunctionValueNode::var("sum".into()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);
        api.store_function(graph, "add".to_string(), None).unwrap();

        // Define the sum_list function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("sum".into()),
            FunctionValueNode::external("add_func".into(), api.get_ptr("add".into()).unwrap()),
            FunctionValueNode::constant("0".into(), IntStored(0)),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Reduce, vec!["add_func".into(), "list".into(), "0".into()], "sum".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["sum".into()]);
        api.store_function(graph, "sum_list".to_string(), None).unwrap();

        // Test the sum_list function.
        let vals = vec![10, 20, 30, 40];
        let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();
        api.store_list(val_symbols, "test_list".to_string()).unwrap();

        api.execute(vec!["sum_list".to_string()], vec!["test_list".to_string()], vec!["sum".to_string()]).unwrap();
        let result = api.get("sum".to_string()).unwrap();

        assert_eq!(result.as_live().as_int().unwrap(), Ok(100));
    }

    fn test_map(vm: &mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
        let mut api = GraphiteApi { vm, symbol_table };

        // Define the double function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num".into()),
            FunctionValueNode::var("double".into()),
            FunctionValueNode::constant("two".into(), IntStored(2)),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Mul, vec!["num".into(), "two".into()], "double".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num".into()], vec!["double".into()]);

        api.store_function(graph, "double".to_string(), None).unwrap();

        // Define the double_list function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("double_list".into()),
            FunctionValueNode::external("double_func".into(), api.get_ptr("double".into()).unwrap()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Map, vec!["double_func".into(), "list".into()], "double_list".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["double_list".into()]);

        api.store_function(graph, "double_list".to_string(), None).unwrap();

        // Test the double_list function.
        let vals = vec![10, 20, 30, 40];
        let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();

        api.store_list(val_symbols, "test_list".to_string()).unwrap();

        api.execute(vec!["double_list".to_string()], vec!["test_list".to_string()], vec!["list_doubled".to_string()]).unwrap();

        let result = api.get("list_doubled".to_string()).unwrap();
        let result_live = result.as_live().as_list().unwrap().unwrap();

        assert_eq!(result_live.len(), 4);

        // define a function to get the values from the list
        let values = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("index".into()),
            FunctionValueNode::var("item".into()),
        ];

        let ops = vec![
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "index".into()], "item".into())
        ];

        let graph = FunctionGraph::new(values, ops, vec!["list".into(), "index".into()], vec!["item".into()]);
        api.store_function(graph, "get_item".to_string(), None).unwrap();

        for i in 0..vals.len() {
            api.store_int(i as i64, "index".to_string()).unwrap();

            api.execute(vec!["get_item".to_string()], vec!["list_doubled".to_string(), "index".into()], vec!["item".to_string()]).unwrap();
            let item = api.get("item".to_string()).unwrap();
            let item = item.as_live().as_int().unwrap().unwrap();

            assert_eq!(item, vals[i] * 2);

            api.drop("index".to_string()).unwrap();
            api.drop("item".to_string()).unwrap();
        }
    }

    fn test_filter<'a>(vm: &'a mut VM) {
        vm.reset();

        let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
        let mut api = GraphiteApi { vm, symbol_table };

        // Define the is_even function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("num".into()),
            FunctionValueNode::constant("two".into(), IntStored(2)),

            FunctionValueNode::var("remainder".into()),
            FunctionValueNode::constant("0".into(), IntStored(0)),
            FunctionValueNode::var("is_even".into()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Mod, vec!["num".into(), "two".into()], "remainder".into()),
            FunctionOpNode::new(OpCode::Equal, vec!["remainder".into(), "0".into()], "is_even".into()),
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num".into()], vec!["is_even".into()]);
        api.store_function(graph, "is_even".to_string(), None).unwrap();

        // Define the filter_list function.
        let values: Vec<FunctionValueNode> = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("filtered_list".into()),
            FunctionValueNode::external("is_even_func".into(), api.get_ptr("is_even".into()).unwrap()),
        ];

        let ops: Vec<FunctionOpNode> = vec![
            FunctionOpNode::new(OpCode::Filter, vec!["is_even_func".into(), "list".into()], "filtered_list".into())
        ];

        let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["list".into()], vec!["filtered_list".into()]);
        api.store_function(graph, "filter_list".to_string(), None).unwrap();

        // Test the filter_list function.
        let vals = vec![5, 10, 15, 20];
        let val_symbols = api.store_multiple(vals.iter().map(|v| StoreOp::StoreInt(*v)).collect(), "num".to_string()).unwrap();
        api.store_list(val_symbols, "test_list".to_string()).unwrap();

        api.execute(vec!["filter_list".to_string()], vec!["test_list".to_string()], vec!["filtered_list".to_string()]).unwrap();
        let result = api.get("filtered_list".to_string()).unwrap();
        let result_live = result.as_live().as_list().unwrap().unwrap();

        assert_eq!(result_live.len(), 2);

        // define a function to get the values from the list
        let values = vec![
            FunctionValueNode::var("list".into()),
            FunctionValueNode::var("index".into()),
            FunctionValueNode::var("item".into()),
        ];

        let ops = vec![
            FunctionOpNode::new(OpCode::GetItem, vec!["list".into(), "index".into()], "item".into())
        ];

        let graph = FunctionGraph::new(values, ops, vec!["list".into(), "index".into()], vec!["item".into()]);
        api.store_function(graph, "get_item".to_string(), None).unwrap();

        for i in 0..result_live.len() {
            api.store_int(i as i64, "index".to_string()).unwrap();

            api.execute(vec!["get_item".to_string()], vec!["filtered_list".to_string(), "index".into()], vec!["item".to_string()]).unwrap();
            let item = api.get("item".to_string()).unwrap();
            let item = item.as_live().as_int().unwrap().unwrap();

            match i {
                0 => assert_eq!(item, 10),
                1 => assert_eq!(item, 20),
                _ => panic!("unexpected index")
            }

            api.drop("index".to_string()).unwrap();
            api.drop("item".to_string()).unwrap();
        }
    }

    fn test_collection(vm: &mut VM) {
        vm.reset();

        let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

        let my_list = vec![10, 20, 30];

        let collection = Collection {
            constants: hashmap!{
            "two".into() => 2.into(),
            "my_list".into() => my_list.iter().map(|v| v.clone().into()).collect::<Vec<CCData>>().into(),
            "my_dict".into() => hashmap!{
                "Hello".to_string() => "World".to_string().into(),
                "Foo".to_string() => "Bar".to_string().into()
            }.into(),
        },
            functions: hashmap! {
        "double".into() => CollectionFunc {graph: CollectionFuncGraph {
            values: vec![
                CFnValueNode::constant("_two".into(), CCData::String("two".to_string())),
                CFnValueNode::var("two".into()),

                CFnValueNode::var("num".into()),
                CFnValueNode::var("doubled".into()),
            ],
            ops: vec![
                // get the 2 const
                FunctionOpNode::new(OpCode::GetItem, vec!["self".into(), "_two".into()], "two".into()),

                // double the number
                FunctionOpNode::new(OpCode::Mul, vec!["num".into(), "two".into()], "doubled".into())
            ],
            input_vals: vec!["num".into()],
            output_vals: vec!["doubled".into()],
        }},
        "double_list".into() => CollectionFunc {graph: CollectionFuncGraph {
            values: vec![
                CFnValueNode::var("double_func".into()),
                CFnValueNode::constant("_double".into(), CCData::String("double".to_string())),
                CFnValueNode::constant("_my_list".into(), CCData::String("my_list".to_string())),

                CFnValueNode::var("my_list".into()),
                CFnValueNode::var("double_list".into()),
            ],
            ops: vec![
                // get the double func and the list
                FunctionOpNode::new(OpCode::GetItem, vec!["self".into(), "_double".into()], "double_func".into()),
                FunctionOpNode::new(OpCode::GetItem, vec!["self".into(), "_my_list".into()], "my_list".into()),

                FunctionOpNode::new(OpCode::Map, vec!["double_func".into(), "my_list".into()], "double_list".into())
            ],
            input_vals: vec![],
            output_vals: vec!["double_list".into()],
        }},
        },
            collections: hashmap!{},
            imports: hashmap!{},
        };

        api.store_collection(collection, "my_collection".to_string()).unwrap();

        api.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();

        let result = api.get("doubled_list".to_string()).unwrap();
        let result = result.as_live().as_list().unwrap().unwrap();

        assert_eq!(result.len(), 3);

        for i in 0..result.len() {
            let item = result.get(i).unwrap();
            let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
            let item = vm.get_ref_value(&item_ref).unwrap();
            let item = item.as_live().as_int().unwrap().unwrap();

            assert_eq!(item, my_list[i] * 2);
        }

    }

    fn test_collection_serialization(vm: &mut VM) {
        vm.reset();

        let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

        let json_collection = r#"{
        "constants": {
            "two": 2,
            "my_list": [10, 20, 30],
            "my_dict": {
                "Hello": "World",
                "Foo": "Bar"
            }
        },
        "functions": {
            "double": {
               "name": "Double",
               "description": "Doubles a number",
               "graph": {
                    "values": [
                        ["_two", "two"],
                        "two",
                        "num",
                        "doubled"
                    ],
                    "ops": [
                        ["GetItem", ["self", "_two"], "two"],
                        ["Mul", ["num", "two"], "doubled"]
                    ],
                    "input_vals": ["num"],
                    "output_vals": ["doubled"]
                }
            },
            "double_list": {
                "name": "Double List",
                "description": "Doubles a list of numbers",
                "graph": {
                    "values": [
                        "double_func",
                        ["_double", "double"],
                        ["_my_list", "my_list"],
                        "my_list",
                        "double_list"
                    ],
                    "ops": [
                        ["GetItem", ["self", "_double"], "double_func"],
                        ["GetItem", ["self", "_my_list"], "my_list"],
                        ["Map", ["double_func", "my_list"], "double_list"]
                    ],
                    "input_vals": [],
                    "output_vals": ["double_list"]
                }
            }
        },
        "collections": {},
        "imports": {}
    }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();

        api.store_collection(collection, "my_collection".to_string()).unwrap();

        api.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();

        let result = api.get("doubled_list".to_string()).unwrap();
        let result = result.as_live().as_list().unwrap().unwrap();

        assert_eq!(result.len(), 3);
        let my_list = vec![10, 20, 30];

        for i in 0..result.len() {
            let item = result.get(i).unwrap();
            let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
            let item = vm.get_ref_value(&item_ref).unwrap();
            let item = item.as_live().as_int().unwrap().unwrap();

            assert_eq!(item, my_list[i] * 2);
        }
    }

    #[test]
    fn run_integration_tests() {
        let mut vm = VM::new(4);

        assert_eq!(vm.object_count(), 0);

        test_dict(&mut vm, hashmap!{
            "Hello".to_string() => "World".to_string(),
            "Foo".to_string() => "Bar".to_string()
        });

        assert_eq!(vm.object_count(), 0);

        test_func_build(&mut vm);

        // println!("state: {:#?}", vm.state);

        assert_eq!(vm.object_count(), 0);

        test_add_func(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_load_fn(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_api(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_calculate_statistics(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_sub_call(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_map(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_reduce(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_filter(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_collection(&mut vm);

        assert_eq!(vm.object_count(), 0);

        test_collection_serialization(&mut vm);

        assert_eq!(vm.object_count(), 0);
    }
}