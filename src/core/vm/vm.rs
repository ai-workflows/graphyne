use std::sync::{Arc, Mutex, RwLock};
use crate::core::data::live::{LiveData};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::{GarbageCollector, GCPointer};
use crate::core::vm::ops::Operation;

macro_rules! execute_cast_op {
    ($self:ident, $arg:expr, $op:ident, $stored_type:ident, $error_msg:expr) => {{
        let arg_value = $arg.get().ok_or("Null pointer exception")?;

        if let Some(result) = arg_value.as_live().$op() {
            return result.map(|live| GCPointer::new(StoredData::$stored_type(live), $self.state.clone()));
        }

        Err($error_msg)
    }};
}

macro_rules! execute_arithmetic_op {
    ($self:ident, $lhs:expr, $rhs:expr, $op:ident, $error_msg:expr) => {{
        let lhs_value = $lhs.get().ok_or("Null pointer exception, LHS")?;
        let rhs_value = $rhs.get().ok_or("Null pointer exception, RHS")?;

        if let Some(result) = lhs_value.as_live().$op(&rhs_value) {
            return result.map(|live| GCPointer::new(live, $self.state.clone()));
        }

        Err($error_msg)
    }};
}

pub struct VM {
    pub state: Arc<RwLock<GarbageCollector<StoredData>>>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            state: Arc::new(RwLock::new(GarbageCollector::new())),
        }
    }

    /// Reset the VM state, clearing all stored data
    pub fn reset(&mut self) {
        self.state.write().unwrap().clear();
    }

    /// Returns the number of objects currently stored in the VM
    pub fn object_count(&self) -> usize {
        self.state.read().unwrap().len()
    }

    pub fn execute_op(&self, operation: Operation) -> ExecResult<GCPointer<StoredData>> {
        match operation {
            Operation::StoreInput(data) => Ok(GCPointer::new(data, self.state.clone())),
            Operation::AsInt(arg) => self.execute_as_int(arg),
            Operation::AsFloat(arg) => self.execute_as_float(arg),
            Operation::AsString(arg) => self.execute_as_string(arg),
            Operation::AsPointer(arg) => self.execute_as_pointer(arg),
            Operation::AsList(arg) => self.execute_as_list(arg),
            Operation::AsDictionary(arg) => self.execute_as_dict(arg),
            Operation::Add(lhs, rhs) => self.execute_add(lhs, rhs),
            Operation::Sub(lhs, rhs) => self.execute_sub(lhs, rhs),
            Operation::Mul(lhs, rhs) => self.execute_mul(lhs, rhs),
            Operation::Div(lhs, rhs) => self.execute_div(lhs, rhs),
            Operation::Length(list) => self.execute_length(list),
            Operation::GetItem(list, index) => self.execute_get_item(list, index),
            Operation::SetItem(list, index, value) => self.execute_set_item(list, index, value),
            Operation::Push(list, value) => self.execute_push(list, value),
            Operation::Remove(list, index) => self.execute_remove(list, index),
        }
    }

    fn execute_as_int(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_int, IntStored, "Cannot cast to int")
    }

    fn execute_as_float(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_float, FloatStored, "Cannot cast to float")
    }

    fn execute_as_string(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_string, StringStored, "Cannot cast to string")
    }

    fn execute_as_pointer(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_pointer, PointerStored, "Cannot cast to pointer")
    }

    fn execute_as_list(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_list, ListStored, "Cannot cast to list")
    }

    fn execute_as_dict(&self, arg: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_cast_op!(self, arg, as_dict, DictStored, "Cannot cast to dict")
    }

    fn execute_length(&self, list: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        let list_value = list.get().ok_or("Null pointer exception")?;

        if let Some(result) = list_value.as_live().op_len() {
            return result.map(|live| GCPointer::new(StoredData::IntStored(live), self.state.clone()));
        }

        Err("Cannot get length")
    }

    fn execute_get_item(&self, collection: GCPointer<StoredData>, index: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        let collection_value = collection.get().ok_or("Null pointer exception")?;
        let index_value = index.get().ok_or("Null pointer exception")?;

        if let Some(result) = collection_value.as_live().op_get_item(&index_value) {
            return result.map(|live| GCPointer::new(live, self.state.clone()));
        }

        Err("Cannot get item")
    }

    fn execute_set_item(&self, collection: GCPointer<StoredData>, index: GCPointer<StoredData>, value: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        let collection_value = collection.get().ok_or("Null pointer exception")?;
        let index_value = index.get().ok_or("Null pointer exception")?;

        if let Some(result) = collection_value.as_live().op_set_item(&index_value, value) {
            return result.map(|live| GCPointer::new(live, self.state.clone()));
        }

        Err("Cannot set item")
    }

    fn execute_push(&self, list: GCPointer<StoredData>, value: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        let list_value = list.get().ok_or("Null pointer exception")?;

        if let Some(result) = list_value.as_live().op_push(value) {
            return result.map(|live| GCPointer::new(live, self.state.clone()));
        }

        Err("Cannot push")
    }

    fn execute_remove(&self, list: GCPointer<StoredData>, index: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        let list_value = list.get().ok_or("Null pointer exception")?;
        let index_value = index.get().ok_or("Null pointer exception")?;

        if let Some(result) = list_value.as_live().op_remove(&index_value) {
            return result.map(|live| GCPointer::new(live, self.state.clone()));
        }

        Err("Cannot remove")
    }

    fn execute_add(&self, lhs: GCPointer<StoredData>, rhs: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_arithmetic_op!(self, lhs, rhs, op_add, "Cannot add")
    }

    fn execute_sub(&self, lhs: GCPointer<StoredData>, rhs: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_arithmetic_op!(self, lhs, rhs, op_sub, "Cannot subtract")
    }

    fn execute_mul(&self, lhs: GCPointer<StoredData>, rhs: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_arithmetic_op!(self, lhs, rhs, op_mul, "Cannot multiply")
    }

    fn execute_div(&self, lhs: GCPointer<StoredData>, rhs: GCPointer<StoredData>) -> ExecResult<GCPointer<StoredData>> {
        execute_arithmetic_op!(self, lhs, rhs, op_div, "Cannot divide")
    }
}