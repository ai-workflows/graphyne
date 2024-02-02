use crate::core::data::live::{LiveData};
use crate::core::ExecResult;
use crate::core::vm::ops::{execute_one_arg_op, execute_three_arg_op, execute_two_arg_op};
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    pub fn execute_add(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_add, lhs, rhs)
    }

    pub fn execute_sub(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_sub, lhs, rhs)
    }

    pub fn execute_mul(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_mul, lhs, rhs)
    }

    pub fn execute_div(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_div, lhs, rhs)
    }

    pub fn execute_mod(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_mod, lhs, rhs)
    }

    pub fn execute_pow(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_pow, lhs, rhs)
    }

    pub fn execute_if(&self, condition: &ValueReference, then: &ValueReference, otherwise: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_three_arg_op!(self, op_if, condition, then, otherwise)
    }

    pub fn execute_not(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_one_arg_op!(self, op_not, arg)
    }

    pub fn execute_and(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_and, lhs, rhs)
    }

    pub fn execute_or(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_or, lhs, rhs)
    }

    pub fn execute_equal(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_eq, lhs, rhs)
    }

    pub fn execute_less_than(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_lt, lhs, rhs)
    }

    pub fn execute_greater_than(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_gt, lhs, rhs)
    }
}