use crate::core::data::live::live_data::NullLive;
use crate::core::data::live::{BoolLive, IntLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for NullLive {
    fn type_tag(&self) -> Type {
        Type::Null
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