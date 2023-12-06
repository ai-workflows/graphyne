use crate::core::data::live::{FloatLive, IntLive, LiveData, StringLive};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for StringLive {
    fn type_tag(&self) -> Type {
        Type::String
    }
    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        todo!()
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        todo!()
    }

    fn as_string(&self) -> Option<ExecResult<StringLive>> {
        Some(Ok(self.clone()))
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        todo!()
    }

    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        todo!()
    }

    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        todo!()
    }

    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        todo!()
    }
}