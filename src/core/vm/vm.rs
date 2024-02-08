#[cfg(test)]
mod tests {
    use crate::core::vm::mmu::store_op::StoreOp;
    use crate::core::vm::VM;
    use crate::core::data::live::{LiveData};
    use crate::core::vm::value_ref::ValueReference;

    #[test]
    fn test_store_pointer() {
        let mut vm = VM::new(2, 2);

        // test storing pointers
        test_store_pointer_helper(&mut vm, "hello");
        assert_eq!(vm.object_count(), 0);

        test_store_pointer_helper(&mut vm, "");
        assert_eq!(vm.object_count(), 0);

    }

    fn test_store_pointer_helper2<'a>(vm: &'a mut VM, value: &str) -> ValueReference {
        let st_results: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(value.to_string())).unwrap();
        let ptr = st_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 1);

        let st_results_meta: Vec<ValueReference> = vm.execute_store(StoreOp::StorePointer(ptr)).unwrap();
        let meta_ptr = st_results_meta.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 2);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        println!("{:?}", vm.state);
        //
        // println!("{:?}", meta_ptr.get().unwrap().as_live().as_pointer().unwrap());

        assert_eq!(vm.get_ref_value(meta_ptr).unwrap().as_live().as_pointer(), Some(Ok(ptr.pointer.clone())));

        // There should be 2 objects in the VM: the string and the pointer
        assert_eq!(vm.object_count(), 2);

        // println!("{:?}", vm.state);

        let result = ptr.clone();

        // there should now be three references to the pointer: ptr, the pointer in the meta, and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 3);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        drop(st_results_meta);

        // there should now be two references to the pointer: ptr and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 2);

        drop(st_results);

        // there should now be one reference to the pointer: the result
        assert_eq!(vm.ref_count(&result).unwrap(), 1);

        return result;
    }

    fn test_store_pointer_helper(vm: &mut VM, value: &str) {
        vm.reset();

        let ptr = test_store_pointer_helper2(vm, value);

        // println!("{:?}", ptr);

        assert_eq!(ptr.vm.ref_count(&ptr).unwrap(), 1);

        // There should be 1 object in the VM: the pointer
        assert_eq!(ptr.vm.object_count(), 1);
    }
}