use crate::runtime::data::live::{BoolLive, DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, ObjectLive, PointerLive, StringLive, TypeLive};
use crate::runtime::ExecResult;

pub fn ptrs_to_int_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&IntLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_int())
        .collect()
}

pub fn ptrs_to_float_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&FloatLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_float())
        .collect()
}

pub fn ptrs_to_string_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&StringLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_string())
        .collect()
}

pub fn ptrs_to_bool_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&BoolLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_bool())
        .collect()
}

pub fn ptrs_to_pointer_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&PointerLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_pointer())
        .collect()
}

pub fn ptrs_to_list_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&ListLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_list())
        .collect()
}

pub fn ptrs_to_dict_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&DictLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_dict())
        .collect()
}

pub fn ptrs_to_func_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&FuncLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_func())
        .collect()
}

pub fn ptrs_to_func_val_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&FuncValLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_func_val())
        .collect()
}

pub fn ptrs_to_func_op_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&FuncOpLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_func_op())
        .collect()
}

pub fn ptrs_to_type_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&TypeLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_type())
        .collect()
}

pub fn ptrs_to_object_list(list: &Vec<PointerLive>) -> ExecResult<Vec<&ObjectLive>> {
    list.iter()
        .map(|ptr| ptr.stored_as_object())
        .collect()
}

