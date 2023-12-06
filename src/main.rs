use std::sync::{Arc, Mutex};
use crate::core::data::stored::StoredData;
use crate::core::gc::{GarbageCollector, GCPointer};

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
    let i: StoredData = StoredData::IntStored(2);
    let j: StoredData = StoredData::IntStored(6);
    let f: StoredData = StoredData::FloatStored(5.5);

    let result = i.__add(&f);
    let result2 = i.__add(&j);
    let result3 = f.__add(&i);

    println!("{:?}", result);
    println!("{:?}", result2);
    println!("{:?}", result3);

    let zero: StoredData = StoredData::IntStored(0);

    let result4 = i.__div(&zero);

    println!("{:?}", result4);

    let result5 = i.__as_float();
    println!("{:?}", result5);

    let gc: Arc<Mutex<GarbageCollector>> = Arc::new(Mutex::new(GarbageCollector::new()));

    test_gc(Arc::clone(&gc));

    println!("{:?}", gc.lock().unwrap().ref_count(0));
}
