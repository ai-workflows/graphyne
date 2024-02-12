pub(crate) mod ops;
pub(crate) mod cast;
pub(crate) mod general;
pub(crate) mod results;
pub(crate) mod collections;
pub(crate) mod types;
pub(crate) mod objects;


macro_rules! execute_one_arg_op {
    ($mmu:ident, $op:ident, $arg:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let arg_value = $mmu.get_ref_value($arg)?;

            arg_value.clone().as_live().$op().map_or_else(
                || $handle_op_null_result($mmu.clone(), arg_value, stringify!($op)),
                |result| $handle_op_result($mmu.clone(), result)
            )
        }
    };
}

macro_rules! execute_two_arg_op {
    ($mmu:ident, $op:ident, $lhs:ident, $rhs:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let lhs_value = $mmu.get_ref_value($lhs)?;
            let rhs_value = $mmu.get_ref_value($rhs)?;

            lhs_value.clone().as_live().$op(&rhs_value).map_or_else(
                || $handle_op_null_result($mmu.clone(), lhs_value, stringify!($op)),
                |result| $handle_op_result($mmu.clone(), result)
            )
        }
    };
}

macro_rules! execute_three_arg_op {
    ($mmu:ident, $op:ident, $arg1:ident, $arg2:ident, $arg3:ident, $handle_op_null_result:ident, $handle_op_result:ident) => {
        {
            let arg1_value = $mmu.get_ref_value($arg1)?;
            let arg2_value = $mmu.get_ref_value($arg2)?;
            let arg3_value = $mmu.get_ref_value($arg3)?;

            arg1_value.clone().as_live().$op(&arg2_value, &arg3_value).map_or_else(
                || $handle_op_null_result($mmu.clone(), arg1_value, stringify!($op)),
                |result| $handle_op_result($mmu.clone(), result)
            )
        }
    };
}

macro_rules! execute_cast_op {
    ($mmu:ident, $arg:ident, $cast_fn:ident, $store_variant:path, $get_stored_type:ident, $store_value:ident) => {
        {
            let arg_value: StoredData = $mmu.get_ref_value($arg).map_err(|msg| msg)?;

            arg_value.clone().as_live().$cast_fn().map_or_else(
                || {
                    let arg_type: TypeLive = match get_stored_type($mmu.clone(), &arg_value) {
                        Ok(type_live) => type_live,
                        Err(msg) => return Err(format!("Cannot cast value to target type with {} (failed to get type of operand: {}) ", stringify!($cast_fn), msg))
                    };
                    Err(format!("Cannot cast {} to target type with {}, operation not supported", arg_type.get_name(), stringify!($cast_fn)))
                },
                |result| {
                    let result_value = result?;
                    let stored_result = $store_variant(result_value);
                    $store_value($mmu.clone(), stored_result)
                }
            )
        }
    };
}

pub(crate) use ops::Operation;
pub(crate) use execute_cast_op;
pub(crate) use execute_one_arg_op;
pub(crate) use execute_two_arg_op;
pub(crate) use execute_three_arg_op;
