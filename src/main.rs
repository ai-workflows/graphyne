use crate::core::data::stored::StoredData;
use crate::core::data::live::LiveData;
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

    assert_eq!(live_sum.as_int().unwrap(), Ok(num1 + num2 as i64));

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


fn main() {
    let mut vm = VM::new();

    test_gc(&mut vm, "Hello World");

    // Make sure all objects were garbage collected since the references went out of scope
    assert_eq!(vm.object_count(), 0);

    test_add_nums(&mut vm, 1, 2);

    assert_eq!(vm.object_count(), 0);

    test_concat_strings(&mut vm, "Hello", "World");

    assert_eq!(vm.object_count(), 0);
}
