use std::collections::HashMap;
use maplit::hashmap;
use crate::core::data::functions::{FuncOp, FuncVal, OpCode};

use crate::core::data::stored::StoredData;
use crate::core::data::live::{FuncOpLive, IntLive, LiveData, PointerLive, StringLive};
use crate::core::gc::GCPointer;
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

mod nodes;
mod core;



fn test_gc(vm: &mut VM, value: &str) {
    vm.reset();

    let mut result = vm.execute_op(Operation::StoreString(value.to_string())).unwrap();

    let ref1 = result.get_mut(0).unwrap();
    let ref2 = ref1.clone();

    let val1 = vm.get_ref_value(ref1).unwrap();
    let val2 = vm.get_ref_value(&ref2).unwrap();

    assert_eq!(val1.as_live().as_string().unwrap(), Ok(value.to_string()));
    assert_eq!(val2.as_live().as_string().unwrap(), Ok(value.to_string()));

    assert_eq!(vm.object_count(), 1);

    assert_eq!(vm.ref_count(ref1).unwrap(), 2);

    drop(result);

    assert_eq!(vm.object_count(), 1);
    assert_eq!(vm.ref_count(&ref2).unwrap(), 1);

    drop(ref2);

    assert_eq!(vm.object_count(), 0);
}

fn test_add_nums(vm: &mut VM, num1: i64, num2: i64) {
    vm.reset();

    let results_1: Vec<ValueReference> = vm.execute_op(Operation::StoreInt(num1)).unwrap();
    let results_2: Vec<ValueReference> = vm.execute_op(Operation::StoreInt(num2)).unwrap();

    let ptr1 = results_1.get(0).unwrap();
    let ptr2 = results_2.get(0).unwrap();

    assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
    assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

    let add = Operation::Add(ptr1, ptr2);

    let add_result = vm.execute_op(add).unwrap();
    let sum_ref = add_result.get(0).unwrap();

    assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

    let sum_value = vm.get_ref_value(sum_ref).unwrap();

    assert_eq!(sum_value.as_live().as_int().unwrap(), Ok(num1 + num2));

    // println!("{:?}", sum_value);

    // There should be 3 objects in the VM: the two ints and the sum
    assert_eq!(vm.object_count(), 3);
}

fn test_concat_strings(vm: &mut VM, str1: &str, str2: &str) {
    vm.reset();

    let st_results1: Vec<ValueReference> = vm.execute_op(Operation::StoreString(str1.to_string())).unwrap();
    let st_results2: Vec<ValueReference> = vm.execute_op(Operation::StoreString(str2.to_string())).unwrap();

    let ptr1 = st_results1.get(0).unwrap();
    let ptr2 = st_results2.get(0).unwrap();

    assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
    assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

    let concat_op = Operation::Add(ptr1, ptr2);

    let concatenated_results: Vec<ValueReference> = vm.execute_op(concat_op).unwrap();
    let concatenated_ref = concatenated_results.get(0).unwrap();

    assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

    let concatenated_value = vm.get_ref_value(concatenated_ref).unwrap();

    // println!("{:?}", concatenated_value);

    assert_eq!(concatenated_value.as_live().as_string().unwrap(), Ok(format!("{}{}", str1, str2)));

    // There should be 3 objects in the VM: the two strings and the concatenated string
    assert_eq!(vm.object_count(), 3);
}

fn test_store_pointer_helper<'a>(vm: &'a mut VM, value: &str) -> ValueReference<'a> {
    let st_results: Vec<ValueReference> = vm.execute_op(Operation::StoreString(value.to_string())).unwrap();
    let ptr = st_results.get(0).unwrap();

    assert_eq!(vm.ref_count(ptr).unwrap(), 1);

    let st_results_meta: Vec<ValueReference> = vm.execute_op(Operation::StorePointer(ptr)).unwrap();
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

fn test_store_pointer(vm: &mut VM, value: &str) {
    vm.reset();

    let ptr = test_store_pointer_helper(vm, value);

    // println!("{:?}", ptr);

    assert_eq!(ptr.vm.ref_count(&ptr).unwrap(), 1);

    // There should be 1 object in the VM: the pointer
    assert_eq!(ptr.vm.object_count(), 1);
}

fn test_combine_lists(vm: &mut VM, l1: Vec<StringLive>, l2: Vec<IntLive>) {
    vm.reset();

    let results1 = l1.iter().map(|s| vm.execute_op(Operation::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
    let results2 = l2.iter().map(|i| vm.execute_op(Operation::StoreInt(i.clone())).unwrap()).collect::<Vec<_>>();

    let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
    let refs2 = results2.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();

    // there should be len(list1) + len(list2) objects in the VM
    assert_eq!(vm.object_count(), l1.len() + l2.len());

    // println!("{:?}", vm.state);

    let st_result1 = vm.execute_op(Operation::StoreList(refs1)).unwrap();
    let st_result2 = vm.execute_op(Operation::StoreList(refs2)).unwrap();

    let list1_ref = st_result1.get(0).unwrap();
    let list2_ref = st_result2.get(0).unwrap();

    // there should be len(list1) + len(list2) + 2 objects in the VM
    assert_eq!(vm.object_count(), l1.len() + l2.len() + 2);

    // println!("{:#?}", vm.state);

    let list1_result = vm.get_ref_value(list1_ref).unwrap();
    let list2_result = vm.get_ref_value(list2_ref).unwrap();

    assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
    assert_eq!(list2_result.as_live().as_list().unwrap().unwrap().len(), l2.len());

    // println!("{:?}", list1_result);
    // println!("{:?}", list2_result);

    let concatenated_result = vm.execute_op(Operation::Add(list1_ref, list2_ref)).unwrap();
    let concatenated_ref = concatenated_result.get(0).unwrap();

    let concatenated = vm.get_ref_value(concatenated_ref).unwrap();

    // println!("CONCATENATED: {:?}", concatenated);
    // println!("{:#?}", vm.state);

    let c_len = concatenated.as_live().as_list().unwrap().unwrap().len();
    assert_eq!(c_len, l1.len() + l2.len());

    for i in 0..c_len {
        let index_result = vm.execute_op(Operation::StoreInt(i as i64)).unwrap();
        let get_op = Operation::GetItem(concatenated_ref, index_result.get(0).unwrap());
        let get_result = vm.execute_op(get_op).unwrap();
        let item_ref = get_result.get(0).unwrap();

        let item = vm.get_ref_value(item_ref).unwrap();
        // println!("ITEM: {:?}", item);

        if i < l1.len() {
            assert_eq!(item.as_live().as_string().unwrap(), Ok(l1[i].clone()));
        } else {
            assert_eq!(item.as_live().as_int().unwrap(), Ok(l2[i - l1.len()].clone()));
        }
    }
}

fn test_list_ops(vm: &mut VM, list: Vec<StringLive>) {
    vm.reset();

    let results = list.iter().map(|s| vm.execute_op(Operation::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
    let refs = results.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();

    // there should be len(list) objects in the VM
    assert_eq!(vm.object_count(), list.len());

    let list_result = vm.execute_op(Operation::StoreList(refs)).unwrap();
    let list_ref = list_result.get(0).unwrap();

    // there should be len(list) + 1 objects in the VM
    assert_eq!(vm.object_count(), list.len() + 1);

    // test list length
    let len_op = Operation::Length(list_ref);

    let len_result = vm.execute_op(len_op).unwrap();
    let len_ref = len_result.get(0).unwrap();
    let len = vm.get_ref_value(len_ref).unwrap().as_live().as_int().unwrap().unwrap();

    assert_eq!(len as usize, list.len());
    assert_eq!(vm.object_count(), list.len() + 2);  // list items + list + length

    // test list get
    for i in 0..list.len() {
        let index_result = vm.execute_op(Operation::StoreInt(i as i64)).unwrap();
        let get_op = Operation::GetItem(list_ref, index_result.get(0).unwrap());
        let get_result = vm.execute_op(get_op).unwrap();
        let item_ref = get_result.get(0).unwrap();

        let item = vm.get_ref_value(item_ref).unwrap();
        assert_eq!(item.as_live().as_string().unwrap(), Ok(list[i].clone()));
    }

    // test list set
    let index_result = vm.execute_op(Operation::StoreInt(0)).unwrap();
    let new_value_result = vm.execute_op(Operation::StoreString("new value".to_string())).unwrap();
    let set_op = Operation::SetItem(list_ref, index_result.get(0).unwrap(), new_value_result.get(0).unwrap());
    let set_result = vm.execute_op(set_op).unwrap();
    let new_list_ref = set_result.get(0).unwrap();

    let new_list = vm.get_ref_value(new_list_ref).unwrap();
    let new_list = new_list.as_live().as_list().unwrap().unwrap();

    assert_eq!(new_list.len(), list.len());

    let get_op = Operation::GetItem(new_list_ref, index_result.get(0).unwrap());
    let get_result = vm.execute_op(get_op).unwrap();
    let item_ref = get_result.get(0).unwrap();
    let new_item = vm.get_ref_value(item_ref).unwrap();

    assert_eq!(new_item.as_live().as_string().unwrap(), Ok("new value".to_string()));

    // test list push
    let push_op = Operation::Push(list_ref, new_value_result.get(0).unwrap());
    let push_result = vm.execute_op(push_op).unwrap();
    let new_list_ref = push_result.get(0).unwrap();

    let new_list = vm.get_ref_value(new_list_ref).unwrap();
    let new_list = new_list.as_live().as_list().unwrap().unwrap();

    assert_eq!(new_list.len(), list.len() + 1);

    let last_index_result = vm.execute_op(Operation::StoreInt(list.len() as i64)).unwrap();
    let get_op = Operation::GetItem(new_list_ref, last_index_result.get(0).unwrap());
    let get_result = vm.execute_op(get_op).unwrap();
    let item_ref = get_result.get(0).unwrap();
    let new_item = vm.get_ref_value(item_ref).unwrap();

    assert_eq!(new_item.as_live().as_string().unwrap(), Ok("new value".to_string()));

    // test list remove
    let remove_op = Operation::Remove(list_ref, index_result.get(0).unwrap());
    let remove_result = vm.execute_op(remove_op).unwrap();
    let new_list_ref = remove_result.get(0).unwrap();

    let new_list = vm.get_ref_value(new_list_ref).unwrap();
    let new_list = new_list.as_live().as_list().unwrap().unwrap();

    assert_eq!(new_list.len(), list.len() - 1);

    for i in 0..new_list.len() {
        let index_result = vm.execute_op(Operation::StoreInt(i as i64)).unwrap();
        let get_op = Operation::GetItem(new_list_ref, index_result.get(0).unwrap());
        let get_result = vm.execute_op(get_op).unwrap();
        let item_ref = get_result.get(0).unwrap();

        let item = vm.get_ref_value(item_ref).unwrap();
        assert_eq!(item.as_live().as_string().unwrap(), Ok(list[i + 1].clone()));
    }
}

fn test_dict(vm: &mut VM, dict: HashMap<StringLive, StringLive>) {
    vm.reset();

    let mut new_dict: HashMap<StringLive, &ValueReference> = HashMap::new();

    let results: HashMap<StringLive, Vec<ValueReference>> = dict.iter().map(|(k, v)| {
        let v_ptr = vm.execute_op(Operation::StoreString(v.clone())).unwrap();
        (k.clone(), v_ptr)
    }).collect();

    for (k, v) in results.iter() {
        new_dict.insert(k.clone(), v.get(0).unwrap());
    }

    // there should be len(dict) objects in the VM
    assert_eq!(vm.object_count(), dict.len());

    let st_d_result = vm.execute_op(Operation::StoreDict(new_dict.clone())).unwrap();
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
        let st_key_result = vm.execute_op(Operation::StoreString(k.clone())).unwrap();

        let get_op = Operation::GetItem(dict_ptr, st_key_result.get(0).unwrap());
        let get_result = vm.execute_op(get_op).unwrap();
        let item_ref = get_result.get(0).unwrap();
        let item = vm.get_ref_value(item_ref).unwrap();
        let item = item.as_live().as_string().unwrap().ok().unwrap();

        assert_eq!(item, v.clone());
    }

    // test dict set
    let st_key_result = vm.execute_op(Operation::StoreString("Hello".to_string())).unwrap();
    let new_value_result = vm.execute_op(Operation::StoreString("Hello World".to_string())).unwrap();

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

fn test_bool(vm: &mut VM) {
    vm.reset();

    let st_true_result = vm.execute_op(Operation::StoreBool(true)).unwrap();
    let st_false_result = vm.execute_op(Operation::StoreBool(false)).unwrap();

    let true_ref = st_true_result.get(0).unwrap();
    let false_ref = st_false_result.get(0).unwrap();

    // test bool not
    let not_result = vm.execute_op(Operation::Not(true_ref)).unwrap();
    let not_result = not_result.get(0).unwrap();
    let not_result = vm.get_ref_value(not_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(not_result, false);

    // test bool and
    let and_op = Operation::And(true_ref, false_ref);
    let and_result: Vec<ValueReference> = vm.execute_op(and_op).unwrap();
    let and_result = and_result.get(0).unwrap();
    let and_result = vm.get_ref_value(and_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(and_result, false);

    // test bool or
    let or_op = Operation::Or(true_ref, false_ref);
    let or_result = vm.execute_op(or_op).unwrap();
    let or_result = or_result.get(0).unwrap();
    let or_result = vm.get_ref_value(or_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(or_result, true);

    // test bool eq
    let eq_op = Operation::Equal(true_ref, false_ref);
    let eq_result = vm.execute_op(eq_op).unwrap();
    let eq_result = eq_result.get(0).unwrap();
    let eq_result = vm.get_ref_value(eq_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(eq_result, false);

    // test greater than
    let st_five_result = vm.execute_op(Operation::StoreInt(5)).unwrap();
    let st_ten_result = vm.execute_op(Operation::StoreInt(10)).unwrap();
    let five_ref = st_five_result.get(0).unwrap();
    let ten_ref = st_ten_result.get(0).unwrap();

    let gt_op = Operation::GreaterThan(ten_ref, five_ref);
    let gt_result = vm.execute_op(gt_op).unwrap();
    let gt_result = gt_result.get(0).unwrap();
    let gt_result = vm.get_ref_value(gt_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(gt_result, true);

    // test less than
    let lt_op = Operation::LessThan(ten_ref, five_ref);
    let lt_result = vm.execute_op(lt_op).unwrap();
    let lt_result = lt_result.get(0).unwrap();
    let lt_result = vm.get_ref_value(lt_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

    assert_eq!(lt_result, false);

    // test if
    let if_op = Operation::If(true_ref, ten_ref, five_ref);
    let if_result = vm.execute_op(if_op).unwrap();
    let if_result = if_result.get(0).unwrap();
    let if_result = vm.get_ref_value(if_result).unwrap().as_live().as_int().unwrap().ok().unwrap();

    assert_eq!(if_result, 10);

    let if_op = Operation::If(false_ref, ten_ref, five_ref);
    let if_result = vm.execute_op(if_op).unwrap();
    let if_result = if_result.get(0).unwrap();
    let if_result = vm.get_ref_value(if_result).unwrap().as_live().as_int().unwrap().ok().unwrap();

    assert_eq!(if_result, 5);
}

fn test_func_build(vm: &mut VM) {
    vm.reset();

    let st_return_val_result = vm.execute_op(Operation::StoreFunctionVal(Vec::new())).unwrap();
    let return_val_ref = st_return_val_result.get(0).unwrap();

    let st_add_buffer_result = vm.execute_op(Operation::CreateBuffer).unwrap();
    let add_op_ref = st_add_buffer_result.get(0).unwrap();

    let st_arg1_result = vm.execute_op(Operation::StoreFunctionVal(vec![add_op_ref])).unwrap();
    let arg1_ref = st_arg1_result.get(0).unwrap();

    let st_arg2_result = vm.execute_op(Operation::StoreFunctionVal(vec![add_op_ref])).unwrap();
    let arg2_ref = st_arg2_result.get(0).unwrap();

    // fill the add buffer with the add op
    let add_op = Operation::StoreFunctionOp(OpCode::Add, vec![arg1_ref, arg2_ref], return_val_ref);
    let fill_add_buffer = Operation::SetBuffer(add_op_ref, add_op.get_stored_data().unwrap());
    vm.execute_op(fill_add_buffer).unwrap();

    // create the function
    let store_func_op = Operation::StoreFunction(vec![arg1_ref, arg2_ref], vec![return_val_ref]);
    let store_func_result = vm.execute_op(store_func_op).unwrap();
    let func_ref = store_func_result.get(0).unwrap();

    // println!("state: {:#?}", vm.state);

    // test calling the func op
    let mut context: HashMap<StringLive, ValueReference> = HashMap::new();
    let arg1_guid = vm.get_ref_value(arg1_ref).unwrap().as_live().as_func_val().unwrap().ok().unwrap().guid;
    let arg2_guid = vm.get_ref_value(arg2_ref).unwrap().as_live().as_func_val().unwrap().ok().unwrap().guid;
    let st_arg1_result = vm.execute_op(Operation::StoreInt(5)).unwrap();
    let st_arg2_result = vm.execute_op(Operation::StoreInt(10)).unwrap();
    context.insert(arg1_guid, st_arg1_result.get(0).unwrap().clone());
    context.insert(arg2_guid, st_arg2_result.get(0).unwrap().clone());

    let add_op_val = vm.get_ref_value(add_op_ref).unwrap().as_live().as_func_op().unwrap().ok().unwrap();
    let call_func_op_rst = vm.handle_call_function_op(&add_op_val, &context);
    println!("call_func_op_rst: {:#?}", call_func_op_rst);
    let call_func_op_rst = call_func_op_rst.unwrap();
    assert_eq!(call_func_op_rst.len(), 1);
    let call_func_op_rst = call_func_op_rst.get(0).unwrap();
    let call_func_op_rst = vm.get_ref_value(call_func_op_rst).unwrap().as_live().as_int().unwrap().ok().unwrap();
    assert_eq!(call_func_op_rst, 15);
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

    test_bool(&mut vm);

    assert_eq!(vm.object_count(), 0);

    test_func_build(&mut vm);

    // println!("state: {:#?}", vm.state);

    assert_eq!(vm.object_count(), 0);
}
