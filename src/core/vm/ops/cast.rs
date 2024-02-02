use crate::core::data::live::{LiveData};
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::ops::execute_cast_op;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    pub fn execute_as_int(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_int, StoredData::IntStored)
    }


    pub fn execute_as_float(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_float, StoredData::FloatStored)
    }

    pub fn execute_as_string(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_string, StoredData::StringStored)
    }

    pub fn execute_as_bool(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_bool, StoredData::BoolStored)
    }

    pub fn execute_as_pointer(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_pointer, StoredData::PointerStored)
    }

    pub fn execute_as_list(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_list, StoredData::ListStored)
    }

    pub fn execute_as_dict(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_dict, StoredData::DictStored)
    }

    pub fn execute_as_type(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_type, StoredData::TypeStored)
    }
}