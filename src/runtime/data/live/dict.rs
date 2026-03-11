use std::sync::Arc;
use crate::runtime::data::live::live_data::{DictLive, TypeLive};
use crate::runtime::data::live::{BoolLive, IntLive, LiveData, PointerLive, StringLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;

impl LiveData for DictLive {
    fn type_of(&self, type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        type_map.get_primitive_type(&TypeLive::Dictionary).map(Ok)
    }

    fn as_dict(&self) -> Option<ExecResult<DictLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => None,
        }
    }

    fn op_len(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.len() as IntLive))
    }

    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let key: StringLive = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };

        Some(match self.get(&key) {
            Some(ptr) => Ok(StoredData::PointerStored(ptr.clone())),
            None => Err(format!("Key {} not found", key)),
        })
    }

    fn op_set_item(&self, index: &StoredData, value: PointerLive) -> Option<ExecResult<StoredData>> {
        // copy the dict
        let mut dict = self.clone();

        let key: StringLive = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };

        // replace the pointer at the index (or create a new one)
        dict.insert(key, value);

        // return the new dict
        Some(Ok(StoredData::DictStored(dict)))
    }
}

// #[cfg(test)]
// mod tests {
//     use std::intermediate::HashMap;
//     use crate::runtime::data::live::{LiveData, StringLive};
//     use crate::runtime::vm::ops::Operation;
//     use crate::runtime::vm::mmu::store_op::StoreOp;
//     use crate::runtime::vm::value_ref::ValueReference;
//     use crate::runtime::vm::VM;
//
//     #[test]
//     fn test_dict() {
//         let mut vm = VM::new(2, 2);
//         let dict = vec![
//             ("Hello".to_string(), "World".to_string()),
//             ("Foo".to_string(), "Bar".to_string()),
//             ("Fizz".to_string(), "Buzz".to_string()),
//         ].into_iter().collect::<HashMap<StringLive, StringLive>>();
//
//         {
//             vm.reset();
//
//             let mut new_dict: HashMap<StringLive, &ValueReference> = HashMap::new();
//
//             let results: HashMap<StringLive, Vec<ValueReference>> = dict.iter().map(|(k, v)| {
//                 let v_ptr = vm.execute_store(StoreOp::StoreString(v.clone())).unwrap();
//                 (k.clone(), v_ptr)
//             }).collect();
//
//             for (k, v) in results.iter() {
//                 new_dict.insert(k.clone(), v.get(0).unwrap());
//             }
//
//             // there should be len(dict) objects in the VM
//             assert_eq!(vm.object_count(), dict.len());
//
//             let st_d_result = vm.execute_store(StoreOp::StoreDict(new_dict.clone())).unwrap();
//             let dict_ptr = st_d_result.get(0).unwrap();
//
//             // there should be len(dict) + 1 objects in the VM
//             assert_eq!(vm.object_count(), dict.len() + 1);
//
//             // test dict length
//             let len_op = Operation::Length(dict_ptr);
//             let len_result = vm.execute_op(len_op).unwrap();
//             let len_ref = len_result.get(0).unwrap();
//             let len = vm.get_ref_value(len_ref).unwrap().as_live().as_int().unwrap().ok().unwrap();
//
//             assert_eq!(len as usize, dict.len());
//
//             // test dict get
//             for (k, v) in dict.iter() {
//                 let st_key_result = vm.execute_store(StoreOp::StoreString(k.clone())).unwrap();
//
//                 let get_op = Operation::GetItem(dict_ptr, st_key_result.get(0).unwrap());
//                 let get_result = vm.execute_op(get_op).unwrap();
//                 let item_ref = get_result.get(0).unwrap();
//                 let item = vm.get_ref_value(item_ref).unwrap();
//                 let item = item.as_live().as_string().unwrap().ok().unwrap();
//
//                 assert_eq!(item, v.clone());
//             }
//
//             // test dict set
//             let st_key_result = vm.execute_store(StoreOp::StoreString("Hello".to_string())).unwrap();
//             let new_value_result = vm.execute_store(StoreOp::StoreString("Hello World".to_string())).unwrap();
//
//             let key_ref = st_key_result.get(0).unwrap();
//             let new_value_ref = new_value_result.get(0).unwrap();
//
//             let set_op = Operation::SetItem(dict_ptr, key_ref, new_value_ref);
//             let set_result = vm.execute_op(set_op).unwrap();
//             let new_dict_ref = set_result.get(0).unwrap();
//
//             let get_result = vm.execute_op(Operation::GetItem(new_dict_ref, key_ref)).unwrap();
//             let item_ref = get_result.get(0).unwrap();
//             let item = vm.get_ref_value(item_ref).unwrap();
//             let item = item.as_live().as_string().unwrap().ok().unwrap();
//
//             assert_eq!(item, "Hello World".to_string());
//         }
//
//         // there should be 0 objects in the VM
//         assert_eq!(vm.object_count(), 0);
//
//     }
// }