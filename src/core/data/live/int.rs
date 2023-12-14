use crate::core::data::live::{FloatLive, IntLive, LiveData};
use crate::core::data::stored::StoredData;
use crate::core::fundamentals::ExecResult;
use crate::core::Type;

macro_rules! checked_arithmetic_int_op {
    ($self:ident, $rhs:ident, $op:tt, $op_name:ident) => {
        match $rhs {
            StoredData::IntStored(rhs) => {
                Some(match $self.$op_name(*rhs) {
                    Some(value) => Ok(StoredData::IntStored(value)),
                    None => Err("Overflow Error".to_string()),
                })
            }
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = $rhs.as_live().as_int();

                cast_result.map(|rhs| match $self.$op_name(rhs?) {
                    Some(value) => Ok(StoredData::IntStored(value)),
                    None => Err("Overflow Error".to_string()),
                })
            }
        }
    };
}

impl LiveData for IntLive {
    fn type_tag(&self) -> Type {
        Type::Integer
    }

    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.clone()))
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(Ok(self.clone() as FloatLive))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() == *rhs))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() == rhs?)))
            }
        }
    }

    fn op_lt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() < *rhs))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() < rhs?)))
            }
        }
    }

    fn op_gt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() > *rhs))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() > rhs?)))
            }
        }
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, +, checked_add)
    }

    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, -, checked_sub)
    }

    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, *, checked_mul)
    }

    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, /, checked_div)
    }
}