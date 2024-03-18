use std::sync::Arc;
use crate::runtime::data::live::{LiveData, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::operator::ops::results::{handle_op_null_result, handle_op_result};

    // pub fn get_primitive_type_ptr(mmu: Arc<MMU>, primitive_type: TypeLive)

pub fn execute_is_null(arg: PointerLive) -> ExecResult<Vec<PointerLive>> {
    arg.as_live().is_null().map_or_else(
        || handle_op_null_result(arg.as_ref(), stringify!($op)),
        |result| handle_op_result(result.map(|value| StoredData::BoolStored(value))))
}

/// Gets the type of the arg and returns a reference to it.
pub fn execute_type_of(arg: PointerLive, static_state: Arc<StaticState>) -> ExecResult<Vec<PointerLive>> {
    arg.as_live().type_of(static_state).map_or_else(
        || handle_op_null_result(arg.as_ref(), stringify!($op)),
        |result| handle_op_result(result.map(|value| StoredData::PointerStored(value))))
}
