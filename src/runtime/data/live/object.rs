use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::data::live::{BoolLive, DictLive, IntLive, LiveData, PointerLive};
use crate::runtime::data::live::live_data::{ObjectLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::static_state::state::StaticState;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub type_ptr: PointerLive,
    pub fields: HashMap<Symbol, PointerLive>
}

impl LiveData for ObjectLive {
    /// Returns a pointer to the type of this data.
    fn type_of(&self, _type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        Some(Ok(self.type_ptr.clone()))
    }

    /// Converts this object's fields to a dictionary.
    fn as_dict(&self) -> Option<ExecResult<DictLive>> {
        Some(Ok(self.fields.clone()))
    }

    fn as_object(&self) -> Option<ExecResult<ObjectLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_len(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.fields.len() as IntLive))
    }

    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let key: Symbol = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };

        return Some(match self.fields.get(&key) {
            Some(ptr) => Ok(StoredData::PointerStored(ptr.clone())),
            None => Err(format!("Key {} not found", key)),
        })
    }

    fn op_set_item(&self, index: &StoredData, value: PointerLive) -> Option<ExecResult<StoredData>> {
        // copy the dict
        let mut dict = self.fields.clone();

        let key: Symbol = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };
        
        match self.fields.get(&key) {
            Some(_ptr) => {
                // replace the pointer at the index (or create a new one)
                dict.insert(key, value);
            },
            None => {
                return Some(Err(format!("Key {} not found", key)))
            }
        }

        // return the new dict
        Some(Ok(StoredData::ObjectStored(ObjectLive {
            type_ptr: self.type_ptr.clone(),
            fields: dict,
        })))
    }

    


}