use std::collections::HashMap;
use maplit::hashmap;

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

    println!("{:?}", vm.state);

    let list1_ptr = vm.execute_op(Operation::StoreInput(StoredData::ListStored(ptrs1))).unwrap();
    let list2_ptr = vm.execute_op(Operation::StoreInput(StoredData::ListStored(ptrs2))).unwrap();

    // there should be len(list1) + len(list2) + 2 objects in the VM
    assert_eq!(vm.object_count(), list1.len() + list2.len() + 2);

    println!("{:?}", vm.state);

    let list1_result = list1_ptr.get().unwrap().as_live().as_list().unwrap();
    let list2_result = list2_ptr.get().unwrap().as_live().as_list().unwrap();

    println!("{:?}", list1_result);
    println!("{:?}", list2_result);

    let concat_op = Operation::Add(list1_ptr.clone(), list2_ptr.clone());

    let concatenated = vm.execute_op(concat_op).unwrap();

    let live_concatenated = concatenated.get().unwrap().as_live().as_list().unwrap().ok().unwrap();

    println!("CONCATENATED: {:?}", live_concatenated);
    println!("{:?}", vm.state);

    assert_eq!(live_concatenated.len(), list1.len() + list2.len());

    for (i, item) in live_concatenated.iter().enumerate() {
        let item = item.get().unwrap();

        if i < list1.len() {
            assert_eq!(item.as_live().as_string().unwrap(), Ok(list1[i].clone()));
        } else {
            assert_eq!(item.as_live().as_int().unwrap(), Ok(list2[i - list1.len()].clone()));
        }
    }
}

fn test_list_ops(vm: &mut VM, list: Vec<StringLive>) {
    vm.reset();

    let ptrs = list.iter().map(|s| vm.execute_op(Operation::StoreInput(StoredData::StringStored(s.clone()))).unwrap()).collect::<Vec<_>>();

    // there should be len(list) objects in the VM
    assert_eq!(vm.object_count(), list.len());

    let list_ptr = vm.execute_op(Operation::StoreInput(StoredData::ListStored(ptrs))).unwrap();

    // there should be len(list) + 1 objects in the VM
    assert_eq!(vm.object_count(), list.len() + 1);

    // test list length
    let len_op = Operation::Length(list_ptr.clone());

    let len = vm.execute_op(len_op).unwrap().get().unwrap().as_live().as_int().unwrap().ok().unwrap();

    assert_eq!(len as usize, list.len());

    // test list get
    for (i, item) in list.iter().enumerate() {
        let index: GCPointer<StoredData> = vm.execute_op(Operation::StoreInput(StoredData::IntStored(i as i64))).unwrap();
        let get_op = Operation::GetItem(list_ptr.clone(), index);

        let get_result = vm.execute_op(get_op).unwrap().get().unwrap().as_live().as_string().unwrap().ok().unwrap();

        assert_eq!(get_result, item.clone());
    }

    // test list set
    let index: GCPointer<StoredData> = vm.execute_op(Operation::StoreInput(StoredData::IntStored(0))).unwrap();
    let new_value: GCPointer<StoredData> = vm.execute_op(Operation::StoreInput(StoredData::StringStored("Hello World".to_string()))).unwrap();
    let set_op = Operation::SetItem(list_ptr.clone(), index.clone(), new_value.clone());
    let new_list = vm.execute_op(set_op).unwrap().get().unwrap().as_live().as_list().unwrap().ok().unwrap();

    assert_eq!(new_list[0].get().unwrap().as_live().as_string().unwrap().ok().unwrap(), "Hello World".to_string());

    // test list push
    let push_op = Operation::Push(list_ptr.clone(), new_value.clone());
    let new_list = vm.execute_op(push_op).unwrap().get().unwrap().as_live().as_list().unwrap().ok().unwrap();

    assert_eq!(new_list.len(), list.len() + 1);
    assert_eq!(new_list[list.len()].get().unwrap().as_live().as_string().unwrap().ok().unwrap(), "Hello World".to_string());

    // test list remove
    let remove_op = Operation::Remove(list_ptr.clone(), index.clone());
    let new_list = vm.execute_op(remove_op).unwrap().get().unwrap().as_live().as_list().unwrap().ok().unwrap();

    assert_eq!(new_list.len(), list.len() - 1);

    for (i, item) in new_list.iter().enumerate() {
        let item = item.get().unwrap();

        if i < index.get().unwrap().as_live().as_int().unwrap().ok().unwrap() as usize {
            assert_eq!(item.as_live().as_string().unwrap(), Ok(list[i].clone()));
        } else {
            assert_eq!(item.as_live().as_string().unwrap(), Ok(list[i + 1].clone()));
        }
    }
}

fn test_dict(vm: &mut VM, dict: HashMap<StringLive, StringLive>) {
    vm.reset();

    // store the values in the VM
    let new_dict: HashMap<StringLive, GCPointer<StoredData>> = dict.iter().map(|(k, v)| {
        let v_ptr = vm.execute_op(Operation::StoreInput(StoredData::StringStored(v.clone()))).unwrap();

        (k.clone(), v_ptr)
    }).collect();

    // there should be len(dict) objects in the VM
    assert_eq!(vm.object_count(), dict.len() );

    let dict_ptr = vm.execute_op(Operation::StoreInput(StoredData::DictStored(new_dict))).unwrap();

    // there should be len(dict) + 1 objects in the VM
    assert_eq!(vm.object_count(), dict.len() + 1);

    // test dict length
    let len_op = Operation::Length(dict_ptr.clone());

    let len = vm.execute_op(len_op).unwrap().get().unwrap().as_live().as_int().unwrap().ok().unwrap();

    assert_eq!(len as usize, dict.len());

    // test dict get
    for (k, v) in dict.iter() {
        let key_ptr = vm.execute_op(Operation::StoreInput(StoredData::StringStored(k.clone()))).unwrap();
        let get_op = Operation::GetItem(dict_ptr.clone(), key_ptr);

        let get_result = vm.execute_op(get_op).unwrap().get().unwrap().as_live().as_string().unwrap().ok().unwrap();

        assert_eq!(get_result, v.clone());
    }

    // test dict set
    let key_ptr = vm.execute_op(Operation::StoreInput(StoredData::StringStored("Hello".to_string()))).unwrap();
    let new_value: GCPointer<StoredData> = vm.execute_op(Operation::StoreInput(StoredData::StringStored("Hello World".to_string()))).unwrap();
    let set_op = Operation::SetItem(dict_ptr.clone(), key_ptr.clone(), new_value.clone());

    let set_result = vm.execute_op(set_op).unwrap();
    let set_result_value = set_result.get().unwrap();
    let set_result_live = set_result_value.as_live();
    let set_result_dict_result = set_result_live.as_dict().unwrap();
    let set_result_dict = set_result_dict_result.ok().unwrap();

    let key_string = key_ptr.get().unwrap().as_live().as_string().unwrap().ok().unwrap();

    assert_eq!(set_result_dict.get(&key_string).unwrap().get().unwrap().as_live().as_string().unwrap().ok().unwrap(), "Hello World".to_string());

    drop(set_result);
    drop(set_result_live);
    drop(set_result_dict);
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

    test_store_pointer(&mut vm, "Hello World");

    assert_eq!(vm.object_count(), 0);

    test_combine_lists(&mut vm, vec!["Hello".to_string(), "World".to_string()], vec![1, 2, 3]);

    assert_eq!(vm.object_count(), 0);

    test_list_ops(&mut vm, vec!["Hello".to_string(), "World".to_string(), "Foo".to_string(), "Bar".to_string()]);

    assert_eq!(vm.object_count(), 0);

    test_dict(&mut vm, hashmap!{
        "Hello".to_string() => "World".to_string(),
        "Foo".to_string() => "Bar".to_string()
    });

    assert_eq!(vm.object_count(), 0);
}
