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