use crate::core::data::live::{LiveData};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::ops::execute_two_arg_op;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    pub fn execute_length(&self, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let list_value: StoredData = self.get_ref_value(list).map_err(|msg| msg)?;

        list_value.clone().as_live().op_len().map_or_else(
            || {
                let arg_type = match self.get_stored_type(&list_value) {
                    Ok(type_live) => type_live,
                    Err(msg) => return Err(format!("Cannot execute operation {} on unknown type (failed to get type of operand: {}) ", stringify!($op), msg))
                };
                Err(format!("Cannot execute op_len on type {}, operation not supported", arg_type.get_name()))
            },
            |result| {
                let result_value = result?;
                let stored_result = StoredData::IntStored(result_value);
                self.store_value(stored_result)
            }
        )
    }

    pub fn execute_get_item(&self, collection: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_get_item, collection, index)
    }

    pub fn execute_set_item(&self, collection: &ValueReference, index: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let collection_val = self.get_ref_value(collection)?;
        let index_val = self.get_ref_value(index)?;
        // The gc will automatically count the cloned pointer once we allocate the new list.
        let val_ptr = value.pointer.clone();

        collection_val.clone().as_live().op_set_item(&index_val, val_ptr).map_or_else(
            || self.handle_op_null_result(collection_val, stringify!($op)),
            |result| self.handle_op_result(result)
        )
    }

    pub fn execute_push(&self, list: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let list_val = self.get_ref_value(list)?;
        // The gc will automatically count the cloned pointer once we allocate the new list.
        let val_ptr = value.pointer.clone();

        list_val.clone().as_live().op_push(val_ptr).map_or_else(
            || self.handle_op_null_result(list_val, stringify!($op)),
            |result| self.handle_op_result(result)
        )
    }

    pub fn execute_remove(&self, list: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_remove, list, index)
    }
}