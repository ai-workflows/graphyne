use std::collections::HashMap;
use crate::runtime::data::live::{BoolLive, FuncLive, FuncOpLive, FuncValLive, LiveData, PointerLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::live::helpers::type_of_helper;
use crate::runtime::data::live::live_data::TypeLive;
use crate::runtime::data::stored::StoredData;

impl LiveData for FuncLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Function, &type_map)
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
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::FunctionVal, &type_map)
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
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::FunctionOp, &type_map)
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