pub(crate) mod val;
pub(crate) mod op;
pub(crate)mod graph;


pub(crate) use val::FunctionValueNode;
pub(crate) use op::FunctionOpNode;
pub(crate) use graph::FunctionGraph;

// #[cfg(test)]
// mod tests {
//     use crate::binder::functions::{FunctionGraph, FunctionOpNode, FunctionValueNode};
//     use crate::runtime::data::functions::OpCode;
//     use crate::runtime::data::live::live_data::LiveData;
//     use crate::runtime::vm::mmu::store_op::StoreOp;
//     use crate::runtime::vm::value_ref::ValueReference;
//     use crate::runtime::vm::VM;
//
//     #[test]
//     fn test_load_fn() {
//         let vm = VM::new(2, 2);
//
//         let values: Vec<FunctionValueNode> = vec![
//             FunctionValueNode::var("num1".into()),
//             FunctionValueNode::var("num2".into()),
//             FunctionValueNode::var("sum".into()),
//         ];
//
//         let ops: Vec<FunctionOpNode> = vec![
//             FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
//         ];
//
//         let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);
//
//
//         let load_result = vm.store_function(&graph, None).unwrap();
//         let fn_ref = load_result.get(0).unwrap().clone();
//
//         let fn_val = vm.get_ref_value(&fn_ref).unwrap().as_live().as_func().unwrap().ok().unwrap();
//
//         let st_arg1_result = vm.execute_store(StoreOp::StoreInt(5)).unwrap();
//         let st_arg2_result = vm.execute_store(StoreOp::StoreInt(10)).unwrap();
//
//         let args: Vec<ValueReference> = vec![st_arg1_result.get(0).unwrap().clone(), st_arg2_result.get(0).unwrap().clone()];
//
//         let call_result = vm.handle_call_function(&fn_val, &args).unwrap();
//         let call_result = vm.get_ref_value(call_result.get(0).unwrap()).unwrap().as_live().as_int().unwrap().ok().unwrap();
//
//         assert_eq!(call_result, 15);
//     }
// }