use crate::core::data::live::{LiveData, TypeLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    pub fn handle_op_null_result(&self, operand: StoredData, op: &str) -> ExecResult<Vec<ValueReference>> {
        let operand_type: TypeLive = match self.get_stored_type(&operand) {
            Ok(type_live) => type_live,
            Err(msg) => return Err(format!("Cannot execute operation {} on unknown type (failed to get type of operand: {}) ", op, msg))
        };

        Err(format!("Cannot execute {} on type {}, operation not supported", op, operand_type.get_name()))
    }

    pub fn handle_op_result(&self, result: ExecResult<StoredData>) -> ExecResult<Vec<ValueReference>> {
        match result {
            // If the result is a pointer, we can convert it directly to a value reference (but it needs to be counted)
            Ok(StoredData::PointerStored(ptr)) => self.value_ref_from_ptr(ptr).map(|value_ref| vec![value_ref]),
            // Otherwise, we need to store the result value and return a reference to it
            Ok(result) => self.store_value(result),
            Err(msg) => Err(msg)
        }
    }

    /// Gets a pointer to the type of stored data.
    pub fn get_stored_type_ptr(&self, arg: &StoredData) -> ExecResult<PointerLive> {
        return match arg.as_live().type_of(&self.primitive_types) {
            Some(Ok(ptr)) => Ok(ptr),
            Some(Err(msg)) => return Err(format!("Could not get type of argument: {}", msg)),
            None => Err("Operation type_of not supported for this value".to_string()),
        };
    }
}