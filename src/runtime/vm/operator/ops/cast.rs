use std::sync::Arc;
use crate::runtime::data::live::{LiveData, PointerLive};
use crate::runtime::data::live::live_data::TypeLive;
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;

macro_rules! execute_cast_op {
    ($arg:ident, $cast_fn:ident, $store_variant:path, $get_stored_type:ident, $store_value:ident) => {
        {
            $arg.as_live().$cast_fn().map_or_else(
                || {
                    let arg_type: TypeLive = $arg.as_ref().type_of()?;
                    Err(format!("Cannot cast {} to target type with {}, operation not supported", arg_type.get_name(), stringify!($cast_fn)))
                },
                |result| {
                    let stored_result = $store_variant(result?);
                    Ok(vec![Arc::new(stored_result)])
                }
            )
        }
    };
}

pub fn execute_as_int(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_int, StoredData::IntStored, get_stored_type, store_value)
}

pub fn execute_as_float(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_float, StoredData::FloatStored, get_stored_type, store_value)
}

pub fn execute_as_string(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_string, StoredData::StringStored, get_stored_type, store_value)
}

pub fn execute_as_bool(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_bool, StoredData::BoolStored, get_stored_type, store_value)
}

pub fn execute_as_pointer(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_pointer, StoredData::PointerStored, get_stored_type, store_value)
}

pub fn execute_as_list(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_list, StoredData::ListStored, get_stored_type, store_value)
}

pub fn execute_as_dict(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_dict, StoredData::DictStored, get_stored_type, store_value)
}

pub fn execute_as_type(arg: Arc<StoredData>) -> ExecResult<Vec<PointerLive>> {
    execute_cast_op!(arg, as_type, StoredData::TypeStored, get_stored_type, store_value)
}