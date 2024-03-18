use std::sync::{Arc};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::operator::ops::cast::{execute_as_bool, execute_as_dict, execute_as_float, execute_as_int, execute_as_list, execute_as_pointer, execute_as_string, execute_as_type};
use crate::runtime::vm::operator::ops::collections::{execute_get_item, execute_length, execute_push, execute_remove, execute_set_item};
use crate::runtime::vm::operator::ops::general::{execute_add, execute_and, execute_div, execute_equal, execute_greater_than, execute_if, execute_less_than, execute_mod, execute_mul, execute_not, execute_or, execute_pow, execute_sub};
use crate::runtime::vm::operator::ops::objects::execute_init;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::operator::ops::types::{execute_is_null, execute_type_of};

pub fn execute_op(operation: Operation, static_state: Arc<StaticState>) -> ExecResult<Vec<Arc<StoredData>>> {
    match operation {
        Operation::TypeOf(arg) => execute_type_of(arg, static_state),
        Operation::AsInt(arg) => execute_as_int(arg),
        Operation::AsFloat(arg) => execute_as_float(arg),
        Operation::AsString(arg) => execute_as_string(arg),
        Operation::AsBool(arg) => execute_as_bool(arg),
        Operation::AsPointer(arg) => execute_as_pointer(arg),
        Operation::AsList(arg) => execute_as_list(arg),
        Operation::AsDictionary(arg) => execute_as_dict(arg),
        Operation::AsType(arg) => execute_as_type(arg),

        Operation::Add(lhs, rhs) => execute_add(lhs, rhs),
        Operation::Sub(lhs, rhs) => execute_sub(lhs, rhs),
        Operation::Mul(lhs, rhs) => execute_mul(lhs, rhs),
        Operation::Div(lhs, rhs) => execute_div(lhs, rhs),
        Operation::Mod(lhs, rhs) => execute_mod(lhs, rhs),
        Operation::Pow(lhs, rhs) => execute_pow(lhs, rhs),

        Operation::Length(list) => execute_length(list),
        Operation::GetItem(list, index) => execute_get_item(list, index),
        Operation::SetItem(list, index, value) => execute_set_item(list, index, value),
        Operation::Push(list, value) => execute_push(list, value),
        Operation::Remove(list, index) => execute_remove(list, index),

        Operation::If(condition, then, otherwise) => execute_if(condition, then, otherwise),
        Operation::Not(arg) => execute_not(arg),
        Operation::And(lhs, rhs) => execute_and(lhs, rhs),
        Operation::Or(lhs, rhs) => execute_or(lhs, rhs),
        Operation::Equal(lhs, rhs) => execute_equal(lhs, rhs),
        Operation::LessThan(lhs, rhs) => execute_less_than(lhs, rhs),
        Operation::GreaterThan(lhs, rhs) => execute_greater_than(lhs, rhs),
        Operation::IsNull(arg) => execute_is_null(arg),

        Operation::Init(obj_type, args) => execute_init(obj_type, args, static_state),

        _ => Err(format!("Operation not implemented: {:?}", operation)),
    }
}