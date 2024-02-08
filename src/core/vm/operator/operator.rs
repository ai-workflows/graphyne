use std::sync::{Arc, RwLock};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::GarbageCollector;
use crate::core::vm::mmu::mmu::MMU;
use crate::core::vm::operator::functions::call::execute_call;
use crate::core::vm::operator::functions::meta::{filter, handle_reduce, map};
use crate::core::vm::operator::ops::cast::{execute_as_bool, execute_as_dict, execute_as_float, execute_as_int, execute_as_list, execute_as_pointer, execute_as_string, execute_as_type};
use crate::core::vm::operator::ops::collections::{execute_get_item, execute_length, execute_push, execute_remove, execute_set_item};
use crate::core::vm::operator::ops::general::{execute_add, execute_and, execute_div, execute_equal, execute_greater_than, execute_if, execute_less_than, execute_mod, execute_mul, execute_not, execute_or, execute_pow, execute_sub};
use crate::core::vm::operator::ops::objects::execute_init;
use crate::core::vm::operator::ops::Operation;
use crate::core::vm::operator::ops::types::{execute_is_null, execute_type_of};
use crate::core::vm::value_ref::ValueReference;

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

        Operation::Call(func, args) => execute_call(mmu, func, args),
        Operation::Map(func, list) => map(mmu, func, list),
        Operation::Reduce(func, list, initial) => handle_reduce(mmu, func, list, initial),
        Operation::Filter(func, list) => filter(mmu, func, list),

        Operation::Init(obj_type, args) => execute_init(mmu, obj_type, args),
    }
}