pub(crate) mod call;
pub(crate) mod meta;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use crate::core::data::functions::OpCode;
    use crate::core::data::live::live_data::LiveData;
    use crate::core::Symbol;
    use crate::core::vm::operator::ops::Operation;
    use crate::core::vm::store::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    #[test]
    fn test_func_build() {
        let vm: &mut VM = &mut VM::new(2, 2);

        {
            let st_return_val_result = vm.execute_store(StoreOp::StoreFunctionVal(Vec::new(), None, false, None)).unwrap();
            let return_val_ref = st_return_val_result.get(0).unwrap();

            let st_add_buffer_result = vm.execute_store(StoreOp::CreateBuffer).unwrap();
            let add_op_ref = st_add_buffer_result.get(0).unwrap();

            let st_arg1_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false, None)).unwrap();
            let arg1_ref = st_arg1_result.get(0).unwrap();

            let st_arg2_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false, None)).unwrap();
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
            // println!("call_func_op_rst: {:#?}", call_func_op_rst);
            let call_func_op_rst = call_func_op_rst.unwrap();
            assert_eq!(call_func_op_rst.len(), 1);
            let call_func_op_rst = call_func_op_rst.get(0).unwrap();
            let call_func_op_rst = vm.get_ref_value(call_func_op_rst).unwrap().as_live().as_int().unwrap().ok().unwrap();
            assert_eq!(call_func_op_rst, 15);
        }

        assert_eq!(vm.object_count(), 0);
    }

    #[test]
    fn test_add_func() {
        let vm: &mut VM = &mut VM::new(2, 2);

        {
            let st_return_val_result = vm.execute_store(StoreOp::StoreFunctionVal(Vec::new(), None, false, None)).unwrap();
            let return_val_ref = st_return_val_result.get(0).unwrap();

            let st_add_buffer_result = vm.execute_store(StoreOp::CreateBuffer).unwrap();
            let add_op_ref = st_add_buffer_result.get(0).unwrap();

            let st_arg1_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false, None)).unwrap();
            let arg1_ref = st_arg1_result.get(0).unwrap();

            let st_arg2_result = vm.execute_store(StoreOp::StoreFunctionVal(vec![add_op_ref], None, false, None)).unwrap();
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

        assert_eq!(vm.object_count(), 0);
    }
}