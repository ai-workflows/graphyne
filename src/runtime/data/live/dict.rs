use std::sync::Arc;
use crate::runtime::data::live::live_data::{DictLive, TypeLive};
use crate::runtime::data::live::{BoolLive, IntLive, LiveData, ObjectLive, PointerLive, StringLive};
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

    fn as_object(&self) -> Option<ExecResult<ObjectLive>> {
        Some(Err("Cannot convert dictionary to object without an associated custom type".to_string()))
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
        let mut dict = self.clone();

        let key: StringLive = match index.as_live().as_string() {
            Some(Ok(key)) => key,
            _ => return Some(Err("Index must be a string".to_string())),
        };

        dict.insert(key, value);

        Some(Ok(StoredData::DictStored(dict)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use crate::runtime::data::live::{DictLive, LiveData};
    use crate::runtime::data::stored::StoredData;

    #[test]
    fn dict_as_object_returns_explicit_error() {
        let dict: DictLive = HashMap::from([(
            "name".to_string(),
            Arc::new(StoredData::StringStored("Ada".to_string())),
        )]);

        let err = dict.as_object().unwrap().unwrap_err();
        assert!(err.contains("Cannot convert dictionary to object"));
    }
}
