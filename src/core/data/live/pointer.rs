use crate::core::data::live::live_data::PointerLive;
use crate::core::data::live::{BoolLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for PointerLive {
    fn type_tag(&self) -> Type {
        Type::Pointer
    }

    fn as_pointer(&self) -> Option<ExecResult<PointerLive>> {
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
}