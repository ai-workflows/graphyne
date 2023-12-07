use std::sync::{Arc, Mutex};
use crate::core::data::stored::StoredData;
use crate::core::data::live::LiveData;
use crate::core::gc::{GarbageCollector, GCObject, GCPointer};
use crate::core::vm::ops::Operation;
use crate::core::vm::VM;

mod nodes;
mod core;

fn test_gc(gc: Arc<Mutex<GarbageCollector>>) {
    let ptr1 = GCPointer::new(StoredData::StringStored("Hello".to_string()), Arc::clone(&gc));
    let ptr2 = ptr1.clone();

    println!("{:?}", ptr1.get());
    println!("{:?}", ptr2.get());
    println!("{:?}", ptr1.ref_count());
    println!("{:?}", ptr2.ref_count());
}

fn test_add_nums(vm: &VM, num1: i64, num2: i64) {
    let ptr1 = vm.execute_op(Operation::StoreLiteral(StoredData::IntStored(num1))).unwrap();
    let ptr2 = vm.execute_op(Operation::StoreLiteral(StoredData::IntStored(num2))).unwrap();
    let add = Operation::Add(ptr1.clone(), ptr2.clone());

    println!("ptr1: {:?}", ptr1.get());
    println!("ptr2: {:?}", ptr2.get());
    println!("add: {:?}", add);

    let sum = vm.execute_op(add).unwrap();

    println!("{:?}", sum.get());

    let sum_value = sum.get().unwrap();

    let live_sum = sum_value.as_live();

    assert_eq!(live_sum.as_int().unwrap(), Ok(num1 + num2 as i64));
}

fn test_concat_strings(vm: &VM, str1: &str, str2: &str) {
    let ptr1 = vm.execute_op(Operation::StoreLiteral(StoredData::StringStored(str1.to_string()))).unwrap();
    let ptr2 = vm.execute_op(Operation::StoreLiteral(StoredData::StringStored(str2.to_string()))).unwrap();
    let concat_op = Operation::Add(ptr1.clone(), ptr2.clone());

    println!("ptr1: {:?}", ptr1.get());
    println!("ptr2: {:?}", ptr2.get());
    println!("concat_op: {:?}", concat_op);

    let concatenated = vm.execute_op(concat_op).unwrap();

    println!("{:?}", concatenated.get());

    let concatenated_value = concatenated.get().unwrap();

    let live_concatenated = concatenated_value.as_live();

    println!("live_concatenated: {:?}", live_concatenated.as_string());

    assert_eq!(live_concatenated.as_string().unwrap(), Ok(format!("{}{}", str1, str2)));
}


fn main() {
    let vm = VM::new();

    test_add_nums(&vm, 1, 2);

    test_concat_strings(&vm, "Hello", "World");
}
