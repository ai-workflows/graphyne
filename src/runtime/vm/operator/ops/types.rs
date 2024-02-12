use std::sync::Arc;
use crate::runtime::data::live::{LiveData, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::ops::results::{get_stored_type_ptr, handle_op_null_result, handle_op_result};

    // pub fn get_primitive_type_ptr(mmu: Arc<MMU>, primitive_type: TypeLive)

pub fn execute_is_null(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let arg_value: StoredData = mmu.get_ref_value(arg).map_err(|msg| msg)?;

    arg_value.clone().as_live().is_null().map_or_else(
        || handle_op_null_result(mmu.clone(), arg_value, stringify!($op)),
        |result| handle_op_result(mmu.clone(), result.map(|value| StoredData::BoolStored(value))))
}

/// Gets the type of the arg and returns a reference to it.
pub fn execute_type_of(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let arg_value: StoredData = mmu.get_ref_value(arg).map_err(|msg| msg)?;

    let res: ExecResult<PointerLive> = get_stored_type_ptr(mmu.clone(), &arg_value);

    handle_op_result(mmu, res.map(|ptr| StoredData::PointerStored(ptr)))
}
