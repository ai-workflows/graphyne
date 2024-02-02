use std::collections::HashMap;
use crate::core::data::live::live_data::{NullLive, TypeLive};
use crate::core::data::live::{BoolLive, IntLive, LiveData, PointerLive};
use crate::core::{ExecResult, Type};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

impl LiveData for NullLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Null, &type_map)
    }

    fn as_null(&self) -> Option<ExecResult<NullLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(true)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::NullStored => Some(Ok(StoredData::BoolStored(true))),
            _ => {
                let cast_result: Option<ExecResult<NullLive>> = rhs.as_live().as_null();

                match cast_result {
                    Some(Ok(_)) => Some(Ok(StoredData::BoolStored(true))),
                    _ => Some(Ok(StoredData::BoolStored(false))),
                }
            }
        }
    }


}