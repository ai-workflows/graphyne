use std::sync::{Arc};
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::ops::cast::{execute_as_bool, execute_as_dict, execute_as_float, execute_as_int, execute_as_list, execute_as_pointer, execute_as_string, execute_as_type};
use crate::runtime::vm::operator::ops::collections::{execute_get_item, execute_length, execute_push, execute_remove, execute_set_item};
use crate::runtime::vm::operator::ops::general::{execute_add, execute_and, execute_div, execute_equal, execute_greater_than, execute_if, execute_less_than, execute_mod, execute_mul, execute_not, execute_or, execute_pow, execute_sub};
use crate::runtime::vm::operator::ops::objects::execute_init;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::operator::ops::types::{execute_is_null, execute_type_of};

pub fn execute_op(mmu: Arc<MMU>, operation: Operation) -> ExecResult<Vec<ValueReference>> {
    match operation {
        Operation::SetBuffer(buffer, value) => mmu.execute_fill_buffer(buffer, value),

        Operation::TypeOf(arg) => execute_type_of(mmu, arg),
        Operation::AsInt(arg) => execute_as_int(mmu, arg),
        Operation::AsFloat(arg) => execute_as_float(mmu, arg),
        Operation::AsString(arg) => execute_as_string(mmu, arg),
        Operation::AsBool(arg) => execute_as_bool(mmu, arg),
        Operation::AsPointer(arg) => execute_as_pointer(mmu, arg),
        Operation::AsList(arg) => execute_as_list(mmu, arg),
        Operation::AsDictionary(arg) => execute_as_dict(mmu, arg),
        Operation::AsType(arg) => execute_as_type(mmu, arg),

        Operation::Add(lhs, rhs) => execute_add(mmu, lhs, rhs),
        Operation::Sub(lhs, rhs) => execute_sub(mmu, lhs, rhs),
        Operation::Mul(lhs, rhs) => execute_mul(mmu, lhs, rhs),
        Operation::Div(lhs, rhs) => execute_div(mmu, lhs, rhs),
        Operation::Mod(lhs, rhs) => execute_mod(mmu, lhs, rhs),
        Operation::Pow(lhs, rhs) => execute_pow(mmu, lhs, rhs),

        Operation::Length(list) => execute_length(mmu, list),
        Operation::GetItem(list, index) => execute_get_item(mmu, list, index),
        Operation::SetItem(list, index, value) => execute_set_item(mmu, list, index, value),
        Operation::Push(list, value) => execute_push(mmu, list, value),
        Operation::Remove(list, index) => execute_remove(mmu, list, index),

        Operation::If(condition, then, otherwise) => execute_if(mmu, condition, then, otherwise),
        Operation::Not(arg) => execute_not(mmu, arg),
        Operation::And(lhs, rhs) => execute_and(mmu, lhs, rhs),
        Operation::Or(lhs, rhs) => execute_or(mmu, lhs, rhs),
        Operation::Equal(lhs, rhs) => execute_equal(mmu, lhs, rhs),
        Operation::LessThan(lhs, rhs) => execute_less_than(mmu, lhs, rhs),
        Operation::GreaterThan(lhs, rhs) => execute_greater_than(mmu, lhs, rhs),
        Operation::IsNull(arg) => execute_is_null(mmu, arg),

        Operation::Init(obj_type, args) => execute_init(mmu, obj_type, args),

        _ => Err(format!("Operation not implemented: {:?}", operation)),
    }
}