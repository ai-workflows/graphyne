use std::sync::Arc;
use crate::runtime::data::live::{LiveData};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::{get_stored_type, MMU, store_value};
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::ops::execute_two_arg_op;
use crate::runtime::vm::operator::ops::results::{handle_op_null_result, handle_op_result};


pub fn execute_length(mmu: Arc<MMU>, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let list_value: Arc<StoredData> = mmu.get_ref_value(list).map_err(|msg| msg)?;

    list_value.clone().as_live().op_len().map_or_else(
        || {
            let arg_type = match get_stored_type(mmu.clone(), &list_value) {
                Ok(type_live) => type_live,
                Err(msg) => return Err(format!("Cannot execute operation {} on unknown type (failed to get type of operand: {}) ", stringify!($op), msg))
            };
            Err(format!("Cannot execute op_len on type {}, operation not supported", arg_type.get_name()))
        },
        |result| {
            let result_value = result?;
            let stored_result = StoredData::IntStored(result_value);
            store_value(mmu.clone(), stored_result)
        }
    )
}

pub fn execute_get_item(mmu: Arc<MMU>, collection: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_get_item, collection, index, handle_op_null_result, handle_op_result)

}

pub fn execute_set_item(mmu: Arc<MMU>, collection: &ValueReference, index: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let collection_val: Arc<StoredData> = mmu.get_ref_value(collection)?;
    let index_val: Arc<StoredData> = mmu.get_ref_value(index)?;
    // The gc will automatically count the cloned pointer once we allocate the new list.
    let val_ptr = value.pointer.clone();

    let res = collection_val.as_live().op_set_item(&index_val, val_ptr);

    res.map_or_else(
        || handle_op_null_result(mmu.clone(), collection_val, stringify!($op)),
        |result| handle_op_result(mmu.clone(), result)
    )
}

pub fn execute_push(mmu: Arc<MMU>, list: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    let list_val = mmu.get_ref_value(list)?;
    // The gc will automatically count the cloned pointer once we allocate the new list.
    let val_ptr = value.pointer.clone();

    let res = list_val.as_live().op_push(val_ptr);

    res.map_or_else(
        || handle_op_null_result(mmu.clone(), list_val, stringify!($op)),
        |result| handle_op_result(mmu.clone(), result)
    )
}

pub fn execute_remove(mmu: Arc<MMU>, list: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_remove, list, index, handle_op_null_result, handle_op_result)
}