use crate::core::data::live::live_data::DictLive;
use crate::core::data::live::{IntLive, LiveData, PointerLive, StringLive};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for DictLive {
    fn type_tag(&self) -> Type {
        Type::Dictionary
    }

    fn as_dict(&self) -> Option<ExecResult<DictLive>> {
        Some(Ok(self.clone()))
    }

    fn op_len(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.len() as IntLive))
    }

    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let key: StringLive = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };

        return Some(match self.get(&key) {
            Some(ptr) => Ok(StoredData::PointerStored(ptr.clone())),
            None => Err("Key not found".to_string()),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::core::data::live::{LiveData, StringLive};
    use crate::core::vm::ops::Operation;
    use crate::core::vm::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    #[test]
    fn test_dict() {
        let mut vm = VM::new(4);
        let dict = vec![
            ("Hello".to_string(), "World".to_string()),
            ("Foo".to_string(), "Bar".to_string()),
            ("Fizz".to_string(), "Buzz".to_string()),
        ].into_iter().collect::<HashMap<StringLive, StringLive>>();

        {
            vm.reset();

            let mut new_dict: HashMap<StringLive, &ValueReference> = HashMap::new();

            let results: HashMap<StringLive, Vec<ValueReference>> = dict.iter().map(|(k, v)| {
                let v_ptr = vm.execute_store(StoreOp::StoreString(v.clone())).unwrap();
                (k.clone(), v_ptr)
            }).collect();

            for (k, v) in results.iter() {
                new_dict.insert(k.clone(), v.get(0).unwrap());
            }

            // there should be len(dict) objects in the VM
            assert_eq!(vm.object_count(), dict.len());

            let st_d_result = vm.execute_store(StoreOp::StoreDict(new_dict.clone())).unwrap();
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
                let st_key_result = vm.execute_store(StoreOp::StoreString(k.clone())).unwrap();

                let get_op = Operation::GetItem(dict_ptr, st_key_result.get(0).unwrap());
                let get_result = vm.execute_op(get_op).unwrap();
                let item_ref = get_result.get(0).unwrap();
                let item = vm.get_ref_value(item_ref).unwrap();
                let item = item.as_live().as_string().unwrap().ok().unwrap();

                assert_eq!(item, v.clone());
            }

            // test dict set
            let st_key_result = vm.execute_store(StoreOp::StoreString("Hello".to_string())).unwrap();
            let new_value_result = vm.execute_store(StoreOp::StoreString("Hello World".to_string())).unwrap();

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

        // there should be 0 objects in the VM
        assert_eq!(vm.object_count(), 0);

    }
}