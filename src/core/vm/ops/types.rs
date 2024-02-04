use crate::core::data::live::{LiveData, PointerLive, TypeLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    // pub fn get_primitive_type_ptr(&self, primitive_type: TypeLive)

    pub fn execute_is_null(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let arg_value: StoredData = self.get_ref_value(arg).map_err(|msg| msg)?;

        arg_value.clone().as_live().is_null().map_or_else(
            || self.handle_op_null_result(arg_value, stringify!($op)),
            |result| self.handle_op_result(result.map(|value| StoredData::BoolStored(value))))
    }

    /// Gets the type of the arg and returns a reference to it.
    pub fn execute_type_of(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let arg_value: StoredData = self.get_ref_value(arg).map_err(|msg| msg)?;

        let res: ExecResult<PointerLive> = self.get_stored_type_ptr(&arg_value);

        self.handle_op_result(res.map(|ptr| StoredData::PointerStored(ptr)))
    }
}
