use crate::runtime::data::live::{LiveData, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::vm::operator::ops::{execute_one_arg_op, execute_three_arg_op, execute_two_arg_op};
use crate::runtime::vm::operator::ops::results::{handle_op_null_result, handle_op_result};

pub fn execute_add(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_add, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_sub(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_sub, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_mul(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_mul, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_div(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_div, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_mod(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_mod, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_pow(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_pow, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_if(condition: PointerLive, then: PointerLive, otherwise: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_three_arg_op!(op_if, condition, then, otherwise, handle_op_null_result, handle_op_result)
}

pub fn execute_not(arg: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_one_arg_op!(op_not, arg, handle_op_null_result, handle_op_result)
}

pub fn execute_and(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_and, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_or(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_or, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_equal(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_eq, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_less_than(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_lt, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_greater_than(lhs: PointerLive, rhs: PointerLive) -> ExecResult<Vec<PointerLive>> {
    execute_two_arg_op!(op_gt, lhs, rhs, handle_op_null_result, handle_op_result)
}