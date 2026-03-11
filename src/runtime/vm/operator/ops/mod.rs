#[allow(clippy::module_inception)]
pub(crate) mod ops;
pub(crate) mod cast;
pub(crate) mod general;
pub(crate) mod results;
pub(crate) mod collections;
pub(crate) mod types;
pub(crate) mod objects;


macro_rules! execute_one_arg_op {
    ($op:ident, $arg:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let arg_value: &StoredData = $arg.as_ref();

            let op_result = arg_value.as_live().$op();

            op_result.map_or_else(
                || $handle_op_null_result(arg_value, stringify!($op)),
                |result| $handle_op_result(result)
            )
        }
    };
}

macro_rules! execute_two_arg_op {
    ($op:ident, $lhs:ident, $rhs:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let lhs_value: &StoredData = $lhs.as_ref();
            let rhs_value: &StoredData = $rhs.as_ref();

            let op_result = lhs_value.as_live().$op(rhs_value);

            op_result.map_or_else(
                || $handle_op_null_result(lhs_value, stringify!($op)),
                |result| $handle_op_result(result)
            )
        }
    };
}

macro_rules! execute_three_arg_op {
    ($op:ident, $arg1:ident, $arg2:ident, $arg3:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let arg1_value: &StoredData = $arg1.as_ref();
            let arg2_value: &StoredData = $arg2.as_ref();
            let arg3_value: &StoredData = $arg3.as_ref();

            let op_result = arg1_value.as_live().$op(arg2_value, arg3_value);

            op_result.map_or_else(
                || $handle_op_null_result(arg1_value, stringify!($op)),
                |result| $handle_op_result(result)
            )
        }
    };
}



pub(crate) use ops::Operation;
pub(crate) use execute_one_arg_op;
pub(crate) use execute_two_arg_op;
pub(crate) use execute_three_arg_op;
