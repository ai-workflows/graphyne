use std::sync::Arc;
use crate::runtime::data::live::live_data::StaticRefLive;
use crate::runtime::data::live::{BoolLive, DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, LiveData, NullLive, ObjectLive, PointerLive, StringLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::static_state::state::StaticState;

fn get_static_value(static_ref: &StaticRefLive) -> ExecResult<&StoredData> {
    match static_ref.get() {
        Some(data) => Ok(data),
        None => Err("Static Reference is not initialized.".to_string())
    }
}

macro_rules! no_arg_op {
    ($name:ident, $ret:ty) => {
        fn $name(&self) -> Option<ExecResult<$ret>> {
            match get_static_value(self) {
                Ok(data) => {
                    data.as_live().$name()
                },
                Err(err) => Some(Err(err))
            }
        }
    };
}

macro_rules! one_arg_op {
    ($name:ident, $ret:ty, $arg:ty) => {
        fn $name(&self, arg: $arg) -> Option<ExecResult<$ret>> {
            match get_static_value(self) {
                Ok(data) => {
                    data.as_live().$name(arg)
                },
                Err(err) => Some(Err(err))
            }
        }
    };
}

macro_rules! two_arg_op {
    ($name:ident, $ret:ty, $arg1:ty, $arg2:ty) => {
        fn $name(&self, arg1: $arg1, arg2: $arg2) -> Option<ExecResult<$ret>> {
            match get_static_value(self) {
                Ok(data) => {
                    data.as_live().$name(arg1, arg2)
                },
                Err(err) => Some(Err(err))
            }
        }
    };
}


/// A static_state reference is a wrapper for normal data so it simply forwards all operations to the underlying data.
impl LiveData for StaticRefLive {
    one_arg_op!(type_of, PointerLive, Arc<StaticState>);
    no_arg_op!(as_int, IntLive);
    no_arg_op!(as_float, FloatLive);
    no_arg_op!(as_string, StringLive);
    no_arg_op!(as_bool, BoolLive);
    no_arg_op!(as_pointer, PointerLive);
    no_arg_op!(as_list, ListLive);
    no_arg_op!(as_dict, DictLive);
    no_arg_op!(as_func, FuncLive);
    no_arg_op!(as_func_val, FuncValLive);
    no_arg_op!(as_func_op, FuncOpLive);
    no_arg_op!(as_null, NullLive);
    no_arg_op!(as_type, TypeLive);
    no_arg_op!(as_object, ObjectLive);

    two_arg_op!(op_if, StoredData, &StoredData, &StoredData);
    no_arg_op!(op_not, StoredData);
    one_arg_op!(op_and, StoredData, &StoredData);
    one_arg_op!(op_or, StoredData, &StoredData);
    one_arg_op!(op_eq, StoredData, &StoredData);
    one_arg_op!(op_lt, StoredData, &StoredData);
    one_arg_op!(op_gt, StoredData, &StoredData);
    no_arg_op!(is_null, BoolLive);

    no_arg_op!(op_len, IntLive);
    one_arg_op!(op_get_item, StoredData, &StoredData);
    two_arg_op!(op_set_item, StoredData, &StoredData, PointerLive);
    one_arg_op!(op_push, StoredData, PointerLive);
    one_arg_op!(op_remove, StoredData, &StoredData);

    one_arg_op!(op_add, StoredData, &StoredData);
    one_arg_op!(op_sub, StoredData, &StoredData);
    one_arg_op!(op_mul, StoredData, &StoredData);
    one_arg_op!(op_div, StoredData, &StoredData);
    one_arg_op!(op_mod, StoredData, &StoredData);
    one_arg_op!(op_pow, StoredData, &StoredData);
}