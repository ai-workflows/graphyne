use crate::core::{ExecResult, Type};
use crate::core::data::live::{FloatLive, IntLive, LiveData};
use crate::core::data::stored::StoredData;

macro_rules! checked_arithmetic_float_op {
    ($self:ident, $rhs:ident, $op:tt) => {
        match $rhs {
            StoredData::FloatStored(rhs) => {
                // Using direct arithmetic operators
                let value = $self $op rhs;
                Some(Ok(StoredData::FloatStored(value)))
            }
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = $rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    let value = $self $op rhs?;
                    Ok(StoredData::FloatStored(value))
                })
            }
        }
    };
}

impl LiveData for FloatLive {
    fn type_tag(&self) -> Type {
        Type::Float
    }

    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(*self as IntLive))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(*self == *rhs))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self == rhs?))
                })
            }
        }
    }

    fn op_lt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(*self < *rhs))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self < rhs?))
                })
            }
        }
    }

    fn op_gt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(*self > *rhs))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self > rhs?))
                })
            }
        }
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(Ok(*self))
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, +)
    }

    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, -)
    }

    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, *)
    }

    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, /)
    }
}