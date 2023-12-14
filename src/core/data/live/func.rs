use crate::core::data::live::{FuncLive, FuncOpLive, FuncValLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for FuncLive {
    fn type_tag(&self) -> Type {
        Type::Function
    }

    fn as_func(&self) -> Option<ExecResult<FuncLive>> {
        Some(Ok(self.clone()))
    }

    #[allow(unused_variables)]
    fn op_call(&self, args: &StoredData) -> Option<ExecResult<StoredData>> {
        todo!()
    }
}

impl LiveData for FuncValLive {
    fn type_tag(&self) -> Type {
        Type::FunctionVal
    }

    fn as_func_val(&self) -> Option<ExecResult<FuncValLive>> {
        Some(Ok(self.clone()))
    }
}

impl LiveData for FuncOpLive {
    fn type_tag(&self) -> Type {
        Type::FunctionOp
    }

    fn as_func_op(&self) -> Option<ExecResult<FuncOpLive>> {
        Some(Ok(self.clone()))
    }
}