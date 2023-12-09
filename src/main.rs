use crate::core::data::stored::StoredData;
use crate::core::data::live::{IntLive, LiveData, PointerLive, StringLive};
use crate::core::gc::GCPointer;
use crate::core::vm::ops::Operation;
use crate::core::vm::VM;

mod nodes;
mod core;

fn test_gc(vm: &mut VM, value: &str) {
    vm.reset();

    let ptr1 = vm.execute_op(Operation::StoreInput(StoredData::StringStored(value.to_string()))).unwrap();
    let ptr2 = ptr1.clone();

    assert_eq!(ptr1.get().unwrap().as_live().as_string().unwrap(), Ok(value.to_string()));
    assert_eq!(ptr2.get().unwrap().as_live().as_string().unwrap(), Ok(value.to_string()));
    assert_eq!(ptr1.ref_count().unwrap(), 2);
    assert_eq!(vm.object_count(), 1);
}

fn test_add_nums(vm: &mut VM, num1: i64, num2: i64) {
    vm.reset();

    let ptr1 = vm.execute_op(Operation::StoreInput(StoredData::IntStored(num1))).unwrap();
    let ptr2 = vm.execute_op(Operation::StoreInput(StoredData::IntStored(num2))).unwrap();
    let add = Operation::Add(ptr1.clone(), ptr2.clone());

    let sum = vm.execute_op(add).unwrap();

    println!("{:?}", sum.get());

    let sum_value = sum.get().unwrap();

    let live_sum = sum_value.as_live();

    assert_eq!(live_sum.as_int().unwrap(), Ok(num1 + num2));

    // There should be 3 objects in the VM: the two ints and the sum
    assert_eq!(vm.object_count(), 3);
}

fn test_concat_strings(vm: &mut VM, str1: &str, str2: &str) {
    vm.reset();

    let ptr1 = vm.execute_op(Operation::StoreInput(StoredData::StringStored(str1.to_string()))).unwrap();
    let ptr2 = vm.execute_op(Operation::StoreInput(StoredData::StringStored(str2.to_string()))).unwrap();
    let concat_op = Operation::Add(ptr1.clone(), ptr2.clone());

    let concatenated = vm.execute_op(concat_op).unwrap();

    println!("{:?}", concatenated.get());

    let concatenated_value = concatenated.get().unwrap();

    let live_concatenated = concatenated_value.as_live();

    assert_eq!(live_concatenated.as_string().unwrap(), Ok(format!("{}{}", str1, str2)));

    // There should be 3 objects in the VM: the two strings and the concatenated string
    assert_eq!(vm.object_count(), 3);
}

fn test_store_pointer_helper(vm: &mut VM, value: &str) -> GCPointer<StoredData> {
    let ptr = vm.execute_op(Operation::StoreInput(StoredData::StringStored(value.to_string()))).unwrap();
    let meta_ptr = vm.execute_op(Operation::StoreInput(StoredData::PointerStored(ptr.clone()))).unwrap();

    println!("{:?}", vm.state);

    println!("{:?}", meta_ptr.get().unwrap().as_live().as_pointer().unwrap());

    assert_eq!(meta_ptr.get().unwrap().as_live().as_pointer().unwrap(), Ok(ptr.clone()));

    // There should be 2 objects in the VM: the string and the pointer
    assert_eq!(vm.object_count(), 2);

    assert_eq!(ptr.ref_count().unwrap(), 2);

    println!("{:?}", vm.state);

    return ptr;
}

fn test_store_pointer(vm: &mut VM, value: &str) {
    vm.reset();

    let ptr = test_store_pointer_helper(vm, value);

    println!("{:?}", ptr);

    assert_eq!(ptr.ref_count().unwrap(), 1);

    // There should be 1 object in the VM: the pointer
    assert_eq!(vm.object_count(), 1);
}

fn test_combine_lists(vm: &mut VM, list1: Vec<StringLive>, list2: Vec<IntLive>) {
    vm.reset();

    let ptrs1 = list1.iter().map(|s| vm.execute_op(Operation::StoreInput(StoredData::StringStored(s.clone()))).unwrap()).collect::<Vec<_>>();
    let ptrs2 = list2.iter().map(|i| vm.execute_op(Operation::StoreInput(StoredData::IntStored(i.clone()))).unwrap()).collect::<Vec<_>>();

    // there should be len(list1) + len(list2) objects in the VM
    assert_eq!(vm.object_count(), list1.len() + list2.len());

    let list1_ptr = vm.execute_op(Operation::StoreInput(StoredData::ListStored(ptrs1))).unwrap();
    let list2_ptr = vm.execute_op(Operation::StoreInput(StoredData::ListStored(ptrs2))).unwrap();

    // there should be len(list1) + len(list2) + 2 objects in the VM
    assert_eq!(vm.object_count(), list1.len() + list2.len() + 2);

    let list1_result = list1_ptr.get().unwrap().as_live().as_list().unwrap();
    let list2_result = list2_ptr.get().unwrap().as_live().as_list().unwrap();

    println!("{:?}", list1_result);
    println!("{:?}", list2_result);

    let concat_op = Operation::Add(list1_ptr.clone(), list2_ptr.clone());

    let concatenated_result = vm.execute_op(concat_op).unwrap();

    let concatenated = concatenated_result.get().unwrap();

    let live_concatenated = concatenated.as_live().as_list().unwrap().ok().unwrap();

    println!("{:?}", live_concatenated);

    assert_eq!(live_concatenated.len(), list1.len() + list2.len());
}

fn main() {
    let mut vm = VM::new();

    // test_gc(&mut vm, "Hello World");
    //
    // // Make sure all objects were garbage collected since the references went out of scope
    // assert_eq!(vm.object_count(), 0);
    //
    // test_add_nums(&mut vm, 1, 2);
    //
    // assert_eq!(vm.object_count(), 0);
    //
    // test_concat_strings(&mut vm, "Hello", "World");
    //
    // assert_eq!(vm.object_count(), 0);

    test_store_pointer(&mut vm, "Hello World");

    assert_eq!(vm.object_count(), 0);

    test_combine_lists(&mut vm, vec!["Hello".to_string(), "World".to_string()], vec![1, 2, 3]);

    assert_eq!(vm.object_count(), 0);
}
