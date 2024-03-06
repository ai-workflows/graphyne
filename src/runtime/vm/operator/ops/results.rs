use std::sync::Arc;
use crate::runtime::data::live::{TypeLive, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;


pub fn handle_op_null_result(operand: &StoredData, op: &str) -> ExecResult<Vec<PointerLive>> {
    let operand_type: TypeLive = operand.type_of()?;

    Err(format!("Cannot execute {} on type {}, operation not supported", op, operand_type.get_name()))
}

pub fn handle_op_result(result: ExecResult<StoredData>) -> ExecResult<Vec<PointerLive>> {
    match result {
        // If the result is a pointer, we can convert it directly to a value reference (but it needs to be counted)
        Ok(StoredData::PointerStored(ptr)) => Ok(vec![ptr]),
        // Otherwise, we need to store the result value and return a reference to it
        Ok(result) => Ok(vec![Arc::new(result)]),
        Err(msg) => Err(msg)
    }
}

// Gets a pointer to the type of stored data.
// pub fn get_stored_type_ptr(arg: &StoredData) -> ExecResult<PointerLive> {
//     return match arg.as_live().type_of(&mmu.primitive_types) {
//         Some(Ok(ptr)) => Ok(ptr),
//         Some(Err(msg)) => return Err(format!("Could not get type of argument: {}", msg)),
//         None => Err("Operation type_of not supported for this value".to_string()),
//     };
// }