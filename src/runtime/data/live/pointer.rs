use std::sync::Arc;
use crate::runtime::data::live::live_data::{PointerLive, TypeLive};
use crate::runtime::data::live::{BoolLive, LiveData};
use crate::runtime::{ExecResult};
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;

impl LiveData for PointerLive {
    fn type_of(&self, type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        type_map.get_primitive_type(&TypeLive::Pointer).map(Ok)
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