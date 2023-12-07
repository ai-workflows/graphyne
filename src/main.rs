use std::sync::{Arc, Mutex};
use crate::core::data::stored::StoredData;
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


fn main() {
    let vm = VM::new();

    let i = vm.execute_op(Operation::StoreLiteral(StoredData::IntStored(1))).unwrap();
    let j = vm.execute_op(Operation::StoreLiteral(StoredData::IntStored(2))).unwrap();
    let add = Operation::Add(i.clone(), j.clone());

    println!("i: {:?}", i.get());
    println!("j: {:?}", j.get());
    println!("add: {:?}", add);

    let sum = vm.execute_op(add).unwrap();

    println!("{:?}", sum.get());

    let h = vm.execute_op(Operation::StoreLiteral(StoredData::StringStored("Hello".to_string()))).unwrap();
    let w = vm.execute_op(Operation::StoreLiteral(StoredData::StringStored("World".to_string()))).unwrap();
    let concat_op = Operation::Add(h.clone(), w.clone());

    println!("h: {:?}", h.get());
    println!("w: {:?}", w.get());
    println!("concat_op: {:?}", concat_op);

    let concatenated = vm.execute_op(concat_op).unwrap();

    println!("{:?}", concatenated.get());


}
