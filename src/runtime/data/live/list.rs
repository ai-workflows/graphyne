use std::sync::Arc;
use crate::runtime::data::live::live_data::{ListLive, TypeLive};
use crate::runtime::data::live::{BoolLive, IntLive, LiveData, PointerLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;

impl LiveData for ListLive {
    fn type_of(&self, type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        type_map.get_primitive_type(&TypeLive::List).map(Ok)
    }

    fn as_list(&self) -> Option<ExecResult<ListLive>> {
        Some(Ok(self.clone()))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::NullStored => Some(Ok(StoredData::BoolStored(false))),
            _ => None,
        }
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_len(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.len() as IntLive))
    }

    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index,
            _ => return Some(Err(format!("Index ({}) for list must be an integer", index.as_live().as_string().unwrap().unwrap()))),
        };

        if index < 0 {
            return Some(Err(format!("Index ({}) for list must be non-negative", index)));
        }

        let index = index as usize;

        if index >= self.len() {
            return Some(Err(format!("Index ({}) for list out of bounds", index)));
        }

        Some(match self.get(index) {
            Some(ptr) => Ok(StoredData::PointerStored(ptr.clone())),
            None => Err(format!("Index ({}) for list out of bounds", index)),
        })
    }

    fn op_set_item(&self, index: &StoredData, value: PointerLive) -> Option<ExecResult<StoredData>> {
        // copy the list
        let mut list = self.clone();

        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index,
            _ => return Some(Err(format!("Index ({}) for list must be an integer", index.as_live().as_string().unwrap().unwrap())))
        };

        if index < 0 {
            return Some(Err(format!("Index ({}) for list must be non-negative", index)));
        }

        let index = index as usize;

        // get the pointer at the index
        match list.get(index) {
            Some(ptr) => ptr,
            None => return Some(Err(format!("Index ({}) for list out of bounds", index)))
        };

        // replace the pointer at the index with the new pointer
        list[index] = value;

        // return the new list
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_push(&self, value: PointerLive) -> Option<ExecResult<StoredData>> {
        let mut list = self.clone();
        list.push(value);
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_remove(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let mut list = self.clone();

        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index,
            _ => return Some(Err(format!("Index ({}) for list must be an integer", index.as_live().as_string().unwrap().unwrap())))
        };

        if index < 0 {
            return Some(Err(format!("Index ({}) for list must be non-negative", index)));
        }

        let index = index as usize;

        // get the pointer at the index
        match list.get(index) {
            Some(ptr) => ptr,
            None => return Some(Err(format!("Index ({}) for list out of bounds", index)))
        };

        // remove the pointer at the index
        list.remove(index);

        // return the new list
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        let mut lhs: ListLive = self.clone();

        // If casting returns none, then casting to ListLive is not implemented for rhs
        // Return None to indicate that the operation is not supported
        let cast_result: ExecResult<ListLive> = rhs.as_live().as_list()?;

        cast_result.map(|rhs| {
            // Iterate over rhs_list and add elements to lhs
            for element in rhs {
                lhs.push(element);
            }
            // Convert ListLive to StoredData and wrap in ExecResult
            Ok(StoredData::ListStored(lhs))
        }).ok()
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::runtime::data::live::live_data::LiveData;
//     use crate::runtime::data::live::{IntLive, StringLive};
//     use crate::runtime::vm::ops::Operation;
//     use crate::runtime::vm::mmu::store_op::StoreOp;
//     use crate::runtime::vm::vm::VM;
//
//     #[test]
//     fn test_combine_lists() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         let l2 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_combine_lists_helper(&mut vm, l1, l2);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         let l2 = vec![];
//
//         test_combine_lists_helper(&mut vm, l1, l2);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![];
//
//         let l2 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_combine_lists_helper(&mut vm, l1, l2);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![];
//
//         let l2 = vec![];
//
//         test_combine_lists_helper(&mut vm, l1, l2);
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_combine_lists_helper(vm: &mut VM, l1: Vec<StringLive>, l2: Vec<IntLive>) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
//         let results2 = l2.iter().map(|i| vm.execute_store(StoreOp::StoreInt(i.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//         let refs2 = results2.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) + len(list2) objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + l2.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//         let st_result2 = vm.execute_store(StoreOp::StoreList(refs2)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//         let list2_ref = st_result2.get(0).unwrap();
//
//         // there should be len(list1) + len(list2) + 2 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + l2.len() + 2);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//         let list2_result = vm.get_ref_value(list2_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//         assert_eq!(list2_result.as_live().as_list().unwrap().unwrap().len(), l2.len());
//
//         // println!("{:?}", list1_result);
//         // println!("{:?}", list2_result);
//
//         let concatenated_result = vm.execute_op(Operation::Add(list1_ref, list2_ref)).unwrap();
//         let concatenated_ref = concatenated_result.get(0).unwrap();
//
//         let concatenated = vm.get_ref_value(concatenated_ref).unwrap();
//
//         // println!("CONCATENATED: {:?}", concatenated);
//         // println!("{:#?}", vm.state);
//
//         let c_len = concatenated.as_live().as_list().unwrap().unwrap().len();
//         assert_eq!(c_len, l1.len() + l2.len());
//
//         for i in 0..c_len {
//             let index_result = vm.execute_store(StoreOp::StoreInt(i as i64)).unwrap();
//             let get_op = Operation::GetItem(concatenated_ref, index_result.get(0).unwrap());
//             let get_result = vm.execute_op(get_op).unwrap();
//             let item_ref = get_result.get(0).unwrap();
//
//             let item = vm.get_ref_value(item_ref).unwrap();
//             // println!("ITEM: {:?}", item);
//
//             if i < l1.len() {
//                 assert_eq!(item.as_live().as_string().unwrap(), Ok(l1[i].clone()));
//             } else {
//                 assert_eq!(item.as_live().as_int().unwrap(), Ok(l2[i - l1.len()].clone()));
//             }
//         }
//     }
//
//     #[test]
//     fn test_list_len() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_len_helper(&mut vm, l1);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_len_helper(&mut vm, l1);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![];
//
//         test_list_len_helper(&mut vm, l1);
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_list_len_helper(vm: &mut VM, l1: Vec<StringLive>) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) objects in the VM
//         assert_eq!(vm.object_count(), l1.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//
//         // there should be len(list1) + 1 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + 1);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         // println!("{:?}", list1_result);
//
//         let len_result = vm.execute_op(Operation::Length(list1_ref)).unwrap();
//         let len_ref = len_result.get(0).unwrap();
//
//         let len = vm.get_ref_value(len_ref).unwrap();
//
//         // println!("LEN: {:?}", len);
//         // println!("{:#?}", vm.state);
//
//         assert_eq!(len.as_live().as_int().unwrap(), Ok(l1.len() as i64));
//     }
//
//     #[test]
//     fn test_list_get_item() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_get_item_helper(&mut vm, l1, 0);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_get_item_helper(&mut vm, l1, 1);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_get_item_helper(&mut vm, l1, 2);
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_list_get_item_helper(vm: &mut VM, l1: Vec<StringLive>, index: usize) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) objects in the VM
//         assert_eq!(vm.object_count(), l1.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//
//         // there should be len(list1) + 1 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + 1);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         // println!("{:?}", list1_result);
//
//         let index_result = vm.execute_store(StoreOp::StoreInt(index as i64)).unwrap();
//         let get_result = vm.execute_op(Operation::GetItem(list1_ref, index_result.get(0).unwrap())).unwrap();
//         let get_ref = get_result.get(0).unwrap();
//
//         let get = vm.get_ref_value(get_ref).unwrap();
//
//         // println!("GET: {:?}", get);
//         // println!("{:#?}", vm.state);
//
//         assert_eq!(get.as_live().as_string().unwrap(), Ok(l1[index].clone()));
//     }
//
//     #[test]
//     fn test_list_set_item() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_set_item_helper(&mut vm, l1, 0, StringLive::from("d"));
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_set_item_helper(&mut vm, l1, 1, StringLive::from("d"));
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             StringLive::from("a"),
//             StringLive::from("b"),
//             StringLive::from("c"),
//         ];
//
//         test_list_set_item_helper(&mut vm, l1, 2, StringLive::from("d"));
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_list_set_item_helper(vm: &mut VM, l1: Vec<StringLive>, index: usize, new_value: StringLive) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreString(s.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) objects in the VM
//         assert_eq!(vm.object_count(), l1.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//
//         // there should be len(list1) + 1 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + 1);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         // println!("{:?}", list1_result);
//
//         let index_result = vm.execute_store(StoreOp::StoreInt(index as i64)).unwrap();
//         let new_value_result = vm.execute_store(StoreOp::StoreString(new_value.clone())).unwrap();
//         let set_result = vm.execute_op(Operation::SetItem(list1_ref, index_result.get(0).unwrap(), new_value_result.get(0).unwrap())).unwrap();
//         let set_ref = set_result.get(0).unwrap();
//
//         let set = vm.get_ref_value(set_ref).unwrap();
//
//         // println!("SET: {:?}", set);
//         // println!("{:#?}", vm.state);
//
//         assert_eq!(set.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         for i in 0..l1.len() {
//             let index_result = vm.execute_store(StoreOp::StoreInt(i as i64)).unwrap();
//             let get_op = Operation::GetItem(set_ref, index_result.get(0).unwrap());
//             let get_result = vm.execute_op(get_op).unwrap();
//             let item_ref = get_result.get(0).unwrap();
//
//             let item = vm.get_ref_value(item_ref).unwrap();
//             // println!("ITEM: {:?}", item);
//
//             if i == index {
//                 assert_eq!(item.as_live().as_string().unwrap(), Ok(new_value.clone()));
//             } else {
//                 assert_eq!(item.as_live().as_string().unwrap(), Ok(l1[i].clone()));
//             }
//         }
//     }
//
//     #[test]
//     fn test_list_push() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_list_push_helper(&mut vm, l1, IntLive::from(4));
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_list_push_helper(&mut vm, l1, IntLive::from(5));
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![];
//
//         test_list_push_helper(&mut vm, l1, IntLive::from(0));
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_list_push_helper(vm: &mut VM, l1: Vec<IntLive>, new_value: IntLive) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreInt(s.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) objects in the VM
//         assert_eq!(vm.object_count(), l1.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//
//         // there should be len(list1) + 1 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + 1);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         // println!("{:?}", list1_result);
//
//         let new_value_result = vm.execute_store(StoreOp::StoreInt(new_value.clone())).unwrap();
//         let push_result = vm.execute_op(Operation::Push(list1_ref, new_value_result.get(0).unwrap())).unwrap();
//         let push_ref = push_result.get(0).unwrap();
//
//         let push = vm.get_ref_value(push_ref).unwrap();
//
//         // println!("PUSH: {:?}", push);
//         // println!("{:#?}", vm.state);
//
//         assert_eq!(push.as_live().as_list().unwrap().unwrap().len(), l1.len() + 1);
//
//         for i in 0..l1.len() {
//             let index_result = vm.execute_store(StoreOp::StoreInt(i as i64)).unwrap();
//             let get_op = Operation::GetItem(push_ref, index_result.get(0).unwrap());
//             let get_result = vm.execute_op(get_op).unwrap();
//             let item_ref = get_result.get(0).unwrap();
//
//             let item = vm.get_ref_value(item_ref).unwrap();
//             // println!("ITEM: {:?}", item);
//
//             assert_eq!(item.as_live().as_int().unwrap(), Ok(l1[i].clone()));
//         }
//
//         let index_result = vm.execute_store(StoreOp::StoreInt(l1.len() as i64)).unwrap();
//         let get_op = Operation::GetItem(push_ref, index_result.get(0).unwrap());
//         let get_result = vm.execute_op(get_op).unwrap();
//         let item_ref = get_result.get(0).unwrap();
//
//         let item = vm.get_ref_value(item_ref).unwrap();
//         // println!("ITEM: {:?}", item);
//
//         assert_eq!(item.as_live().as_int().unwrap(), Ok(new_value.clone()));
//     }
//
//     #[test]
//     fn test_list_remove() {
//         let mut vm = VM::new(2, 2);
//
//         let l1 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_list_remove_helper(&mut vm, l1, 0);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_list_remove_helper(&mut vm, l1, 1);
//         assert_eq!(vm.object_count(), 0);
//
//         let l1 = vec![
//             IntLive::from(1),
//             IntLive::from(2),
//             IntLive::from(3),
//         ];
//
//         test_list_remove_helper(&mut vm, l1, 2);
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     fn test_list_remove_helper(vm: &mut VM, l1: Vec<IntLive>, index: usize) {
//         vm.reset();
//
//         let results1 = l1.iter().map(|s| vm.execute_store(StoreOp::StoreInt(s.clone())).unwrap()).collect::<Vec<_>>();
//
//         let refs1 = results1.iter().map(|r| r.get(0).unwrap()).collect::<Vec<_>>();
//
//         // there should be len(list1) objects in the VM
//         assert_eq!(vm.object_count(), l1.len());
//
//         // println!("{:?}", vm.state);
//
//         let st_result1 = vm.execute_store(StoreOp::StoreList(refs1)).unwrap();
//
//         let list1_ref = st_result1.get(0).unwrap();
//
//         // there should be len(list1) + 1 objects in the VM
//         assert_eq!(vm.object_count(), l1.len() + 1);
//
//         // println!("{:#?}", vm.state);
//
//         let list1_result = vm.get_ref_value(list1_ref).unwrap();
//
//         assert_eq!(list1_result.as_live().as_list().unwrap().unwrap().len(), l1.len());
//
//         // println!("{:?}", list1_result);
//
//         let index_result = vm.execute_store(StoreOp::StoreInt(index as i64)).unwrap();
//         let remove_result = vm.execute_op(Operation::Remove(list1_ref, index_result.get(0).unwrap())).unwrap();
//         let remove_ref = remove_result.get(0).unwrap();
//
//         let remove = vm.get_ref_value(remove_ref).unwrap();
//
//         // println!("REMOVE: {:?}", remove);
//         // println!("{:#?}", vm.state);
//
//         assert_eq!(remove.as_live().as_list().unwrap().unwrap().len(), l1.len() - 1);
//
//         // verify that the removed item is not in the list
//         for i in 0..l1.len() {
//             if i == index {
//                 continue;
//             }
//
//             let index_result = vm.execute_store(StoreOp::StoreInt(if i > index { i as i64 - 1 } else { i as i64 })).unwrap();
//             let get_op = Operation::GetItem(remove_ref, index_result.get(0).unwrap());
//             let get_result = vm.execute_op(get_op).unwrap();
//             let item_ref = get_result.get(0).unwrap();
//
//             let item = vm.get_ref_value(item_ref).unwrap();
//             // println!("ITEM: {:?}", item);
//
//             assert_eq!(item.as_live().as_int().unwrap(), Ok(l1[i].clone()));
//         }
//     }
// }