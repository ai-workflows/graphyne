use std::sync::{Arc, Mutex};
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
    pub state: Arc<Mutex<GarbageCollector>>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            state: Arc::new(Mutex::new(GarbageCollector::new())),
        }
    }

    pub fn execute_op(&self, operation: Operation) -> ExecResult<GCPointer<StoredData>> {
        match operation {
            Operation::StoreLiteral(data) => Ok(GCPointer::new(data, self.state.clone())),
            Operation::AsInt(arg) => self.execute_as_int(arg),
            Operation::AsFloat(arg) => self.execute_as_float(arg),
            Operation::AsString(arg) => self.execute_as_string(arg),
            Operation::Add(lhs, rhs) => self.execute_add(lhs, rhs),
            Operation::Sub(lhs, rhs) => self.execute_sub(lhs, rhs),
            Operation::Mul(lhs, rhs) => self.execute_mul(lhs, rhs),
            Operation::Div(lhs, rhs) => self.execute_div(lhs, rhs),
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