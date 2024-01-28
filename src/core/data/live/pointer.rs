use std::collections::HashMap;
use crate::core::data::live::live_data::{PointerLive, TypeLive};
use crate::core::data::live::{BoolLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::stored::StoredData;

impl LiveData for PointerLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, usize>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Pointer, &type_map)
    }

    fn as_pointer(&self) -> Option<ExecResult<PointerLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }
    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::PointerStored(ptr) => Some(Ok(StoredData::BoolStored(BoolLive::from(self == ptr)))),
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => None,
        }
    }
}