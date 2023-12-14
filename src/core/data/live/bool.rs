use crate::core::data::live::live_data::BoolLive;
use crate::core::data::live::LiveData;
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for BoolLive {
    fn type_tag(&self) -> Type {
        Type::Boolean
    }

    fn as_bool(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(self.clone()))
    }

    fn op_if(&self, then: &StoredData, otherwise: &StoredData) -> Option<ExecResult<StoredData>> {
        if *self {
            Some(Ok(then.clone()))
        } else {
            Some(Ok(otherwise.clone()))
        }
    }

    fn op_not(&self) -> Option<ExecResult<StoredData>> {
        Some(Ok(StoredData::BoolStored(!*self)))
    }

    fn op_and(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self && *rhs))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self && rhs?))
                })
            }
        }
    }

    fn op_or(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self || *rhs))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self || rhs?))
                })
            }
        }
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self == *rhs))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self == rhs?))
                })
            }
        }
    }




}