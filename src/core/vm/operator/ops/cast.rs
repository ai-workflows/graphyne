use std::sync::Arc;
use crate::core::data::live::{LiveData};
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::mmu::mmu::{get_stored_type, MMU, store_value};
use crate::core::vm::operator::ops::execute_cast_op;
use crate::core::vm::value_ref::ValueReference;

pub fn execute_as_int(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_int, StoredData::IntStored, get_stored_type, store_value)
}


pub fn execute_as_float(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_float, StoredData::FloatStored, get_stored_type, store_value)
}

pub fn execute_as_string(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_string, StoredData::StringStored, get_stored_type, store_value)
}

pub fn execute_as_bool(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_bool, StoredData::BoolStored, get_stored_type, store_value)
}

pub fn execute_as_pointer(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_pointer, StoredData::PointerStored, get_stored_type, store_value)
}

pub fn execute_as_list(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_list, StoredData::ListStored, get_stored_type, store_value)
}

pub fn execute_as_dict(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_dict, StoredData::DictStored, get_stored_type, store_value)
}

pub fn execute_as_type(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_cast_op!(mmu, arg, as_type, StoredData::TypeStored, get_stored_type, store_value)
}