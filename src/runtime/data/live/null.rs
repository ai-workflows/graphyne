use std::sync::Arc;
use crate::runtime::data::live::live_data::{NullLive, TypeLive};
use crate::runtime::data::live::{BoolLive, LiveData, PointerLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;

impl LiveData for NullLive {
    fn type_of(&self, type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        type_map.get_primitive_type(&TypeLive::Null).map(|r| Ok(r))
    }

    fn as_null(&self) -> Option<ExecResult<NullLive>> {
        Some(Ok(()))
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

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(true)))
    }


}