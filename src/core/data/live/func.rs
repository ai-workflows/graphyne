use crate::core::data::live::{BoolLive, FuncLive, FuncOpLive, FuncValLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for FuncLive {
    fn type_tag(&self) -> Type {
        Type::Function
    }

    fn as_func(&self) -> Option<ExecResult<FuncLive>> {
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

impl LiveData for FuncValLive {
    fn type_tag(&self) -> Type {
        Type::FunctionVal
    }

    fn as_func_val(&self) -> Option<ExecResult<FuncValLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::NullStored => Some(Ok(StoredData::BoolStored(false))),
            _ => None,
        }
    }
}

impl LiveData for FuncOpLive {
    fn type_tag(&self) -> Type {
        Type::FunctionOp
    }

    fn as_func_op(&self) -> Option<ExecResult<FuncOpLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::NullStored => Some(Ok(StoredData::BoolStored(false))),
            _ => None,
        }
    }
}