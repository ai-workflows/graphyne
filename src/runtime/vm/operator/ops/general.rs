use std::sync::Arc;
use crate::runtime::data::live::{LiveData};
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::ops::{execute_one_arg_op, execute_three_arg_op, execute_two_arg_op};
use crate::runtime::vm::operator::ops::results::{handle_op_null_result, handle_op_result};

pub fn execute_add(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_add, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_sub(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_sub, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_mul(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_mul, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_div(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_div, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_mod(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_mod, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_pow(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_pow, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_if(mmu: Arc<MMU>, condition: &ValueReference, then: &ValueReference, otherwise: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_three_arg_op!(mmu, op_if, condition, then, otherwise, handle_op_null_result, handle_op_result)
}

pub fn execute_not(mmu: Arc<MMU>, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_one_arg_op!(mmu, op_not, arg, handle_op_null_result, handle_op_result)
}

pub fn execute_and(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_and, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_or(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_or, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_equal(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_eq, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_less_than(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_lt, lhs, rhs, handle_op_null_result, handle_op_result)
}

pub fn execute_greater_than(mmu: Arc<MMU>, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
    execute_two_arg_op!(mmu, op_gt, lhs, rhs, handle_op_null_result, handle_op_result)
}