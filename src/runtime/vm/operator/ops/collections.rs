use std::sync::Arc;
use crate::runtime::data::live::{LiveData, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::vm::operator::ops::execute_two_arg_op;
use crate::runtime::vm::operator::ops::results::{handle_op_null_result, handle_op_result};


pub fn execute_length(list: PointerLive) -> ExecResult<Vec<PointerLive>> {    
    let list_value = list.as_ref();

    list_value.as_live().op_len().map_or_else(
        || {
            handle_op_null_result(list_value, "op_len")
        },
        |result| {
            match result {
                Ok(i) => Ok(vec![Arc::new(StoredData::IntStored(i))]),
                Err(msg) => Err(msg)
            }
        }
    )
}

pub fn execute_get_item(collection: PointerLive, index: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_get_item, collection, index, handle_op_null_result, handle_op_result)

}

pub fn execute_set_item(collection: PointerLive, index: PointerLive, value: PointerLive) -> ExecResult<Vec<PointerLive>> {
    let collection_val: &StoredData = collection.as_ref();
    let index_val: &StoredData = index.as_ref();

    let res = collection_val.as_live().op_set_item(&index_val, value);

    res.map_or_else(
        || handle_op_null_result(collection_val, "set_item"),
        |result| handle_op_result(result)
    )
}

pub fn execute_push(list: PointerLive, value: PointerLive) -> ExecResult<Vec<PointerLive>> {
    let list_val = list.as_ref();

    let res = list_val.as_live().op_push(value);

    res.map_or_else(
        || handle_op_null_result(list_val, "op_push"),
        |result| handle_op_result(result)
    )
}

pub fn execute_remove(list: PointerLive, index: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_remove, list, index, handle_op_null_result, handle_op_result)
}

#[cfg(test)]
mod tests {
    use super::execute_set_item;
    use crate::runtime::data::stored::StoredData;

    #[test]
    fn set_item_reports_the_correct_operation_name_for_unsupported_types() {
        let result = execute_set_item(
            std::sync::Arc::new(StoredData::IntStored(1)),
            std::sync::Arc::new(StoredData::StringStored("key".to_string())),
            std::sync::Arc::new(StoredData::IntStored(2)),
        );

        let error = result.expect_err("expected set_item on int to fail");
        assert!(error.contains("set_item"));
        assert!(!error.contains("$op"));
    }
}