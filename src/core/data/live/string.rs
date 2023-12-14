use crate::core::data::live::{FloatLive, IntLive, LiveData, StringLive};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for StringLive {
    fn type_tag(&self) -> Type {
        Type::String
    }
    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(self.parse::<IntLive>().map_err(|_| "Error parsing int from string".to_string()))
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(self.parse::<FloatLive>().map_err(|_| "Error parsing float from string".to_string()))
    }

    fn as_string(&self) -> Option<ExecResult<StringLive>> {
        Some(Ok(self.clone()))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::StringStored(rhs) => Some(Ok(StoredData::BoolStored(*self == *rhs))),
            _ => {
                let cast_result: Option<ExecResult<StringLive>> = rhs.as_live().as_string();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(*self == rhs?)))
            }
        }
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        // concat
        match rhs {
            StoredData::StringStored(rhs) => {
                Some(Ok(StoredData::StringStored(self.clone() + rhs)))
            }
            _ => {
                let cast_result: Option<ExecResult<StringLive>> = rhs.as_live().as_string();

                cast_result.map(|rhs| {
                    Ok(StoredData::StringStored(self.clone() + &rhs?))
                })
            }
        }
    }
}