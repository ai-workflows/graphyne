use std::sync::Arc;
use crate::runtime::data::live::{LiveData, TypeLive, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::{get_stored_type, MMU, store_value, value_ref_from_ptr};
use crate::runtime::mmu::value_ref::ValueReference;


pub fn handle_op_null_result(mmu: Arc<MMU>, operand: Arc<StoredData>, op: &str) -> ExecResult<Vec<ValueReference>> {
    let operand_type: TypeLive = match get_stored_type(mmu, &operand) {
        Ok(type_live) => type_live,
        Err(msg) => return Err(format!("Cannot execute operation {} on unknown type (failed to get type of operand: {}) ", op, msg))
    };

    Err(format!("Cannot execute {} on type {}, operation not supported", op, operand_type.get_name()))
}

pub fn handle_op_result(mmu: Arc<MMU>, result: ExecResult<StoredData>) -> ExecResult<Vec<ValueReference>> {
    match result {
        // If the result is a pointer, we can convert it directly to a value reference (but it needs to be counted)
        Ok(StoredData::PointerStored(ptr)) => value_ref_from_ptr(mmu, ptr).map(|value_ref| vec![value_ref]),
        // Otherwise, we need to store the result value and return a reference to it
        Ok(result) => store_value(mmu, result),
        Err(msg) => Err(msg)
    }
}

/// Gets a pointer to the type of stored data.
pub fn get_stored_type_ptr(mmu: Arc<MMU>, arg: &StoredData) -> ExecResult<PointerLive> {
    return match arg.as_live().type_of(&mmu.primitive_types) {
        Some(Ok(ptr)) => Ok(ptr),
        Some(Err(msg)) => return Err(format!("Could not get type of argument: {}", msg)),
        None => Err("Operation type_of not supported for this value".to_string()),
    };
}