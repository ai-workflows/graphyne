use std::sync::{Arc, RwLock};
use crate::core::data::live::{LiveData, StringLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::{GarbageCollector, GCPointer};
use crate::core::vm::ops::Operation;
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;

macro_rules! execute_cast_op {
    ($self:ident, $arg:ident, $cast_fn:ident, $store_variant:path) => {
        {
            let arg_value: StoredData = $self.get_ref_value($arg).map_err(|msg| msg)?;

            arg_value.clone().as_live().$cast_fn().map_or_else(
                || {
                    let arg_type: StringLive = arg_value.as_live().type_code().unwrap_or_else(|_| "unknown".to_string());
                    Err(format!("Cannot cast {} to target type, operation not supported", arg_type))
                },
                |result| {
                    let result_value = result?;
                    let stored_result = $store_variant(result_value);
                    $self.store_value(stored_result)
                }
            )
        }
    };
}

macro_rules! execute_one_arg_op {
    ($self:ident, $op:ident, $arg:ident) => {
        {
            let arg_value = $self.get_ref_value($arg)?;

            arg_value.clone().as_live().$op().map_or_else(
                || $self.handle_op_null_result(arg_value, stringify!($op)),
                |result| $self.handle_op_result(result)
            )
        }
    };
}

macro_rules! execute_two_arg_op {
    ($self:ident, $op:ident, $lhs:ident, $rhs:ident) => {
        {
            let lhs_value = $self.get_ref_value($lhs)?;
            let rhs_value = $self.get_ref_value($rhs)?;

            lhs_value.clone().as_live().$op(&rhs_value).map_or_else(
                || $self.handle_op_null_result(lhs_value, stringify!($op)),
                |result| $self.handle_op_result(result)
            )
        }
    };
}

macro_rules! execute_three_arg_op {
    ($self:ident, $op:ident, $arg1:ident, $arg2:ident, $arg3:ident) => {
        {
            let arg1_value = $self.get_ref_value($arg1)?;
            let arg2_value = $self.get_ref_value($arg2)?;
            let arg3_value = $self.get_ref_value($arg3)?;

            arg1_value.clone().as_live().$op(&arg2_value, &arg3_value).map_or_else(
                || $self.handle_op_null_result(arg1_value, stringify!($op)),
                |result| $self.handle_op_result(result)
            )
        }
    };
}

#[derive(Debug)]
pub struct VM {
    pub state: Arc<RwLock<GarbageCollector<StoredData>>>,
    pub thread_pool: rayon::ThreadPool,
}

impl VM {
    pub fn new(num_threads: usize) -> Self {
        VM {
            state: Arc::new(RwLock::new(GarbageCollector::new())),
            thread_pool: rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap(),
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

    pub fn execute_store(&self, operation: StoreOp) -> ExecResult<Vec<ValueReference>> {
        return match operation {
            StoreOp::StoreInt(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFloat(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreString(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreBool(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StorePointer(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreList(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreDict(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunction(_, _, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionVal(_, _ ,_, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionOp(_, _, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionGraph(func, class_context) => self.store_function(&func, class_context),
            StoreOp::CreateBuffer => self.store_value(StoredData::NullStored),
        };
    }

    pub fn execute_op(&self, operation: Operation) -> ExecResult<Vec<ValueReference>> {
        match operation {
            Operation::SetBuffer(buffer, value) => self.execute_fill_buffer(buffer, value),
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
            Operation::Mod(lhs, rhs) => self.execute_mod(lhs, rhs),
            Operation::Pow(lhs, rhs) => self.execute_pow(lhs, rhs),
            Operation::Length(list) => self.execute_length(list),
            Operation::GetItem(list, index) => self.execute_get_item(list, index),
            Operation::SetItem(list, index, value) => self.execute_set_item(list, index, value),
            Operation::Push(list, value) => self.execute_push(list, value),
            Operation::Remove(list, index) => self.execute_remove(list, index),
            Operation::AsBool(arg) => self.execute_as_bool(arg),
            Operation::If(condition, then, otherwise) => self.execute_if(condition, then, otherwise),
            Operation::Not(arg) => self.execute_not(arg),
            Operation::And(lhs, rhs) => self.execute_and(lhs, rhs),
            Operation::Or(lhs, rhs) => self.execute_or(lhs, rhs),
            Operation::Equal(lhs, rhs) => self.execute_equal(lhs, rhs),
            Operation::LessThan(lhs, rhs) => self.execute_less_than(lhs, rhs),
            Operation::GreaterThan(lhs, rhs) => self.execute_greater_than(lhs, rhs),
            Operation::IsNull(arg) => self.execute_is_null(arg),
            Operation::Call(func, args) => self.execute_call(func, args),
            Operation::Map(func, list) => self.map(func, list),
            Operation::Reduce(func, list, initial) => self.handle_reduce(func, list, initial),
            Operation::Filter(func, list) => self.filter(func, list),
        }
    }

    /// Gets a copy of the stored data referenced by the given value reference.
    pub fn get_ref_value(&self, arg: &ValueReference) -> ExecResult<StoredData> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = match self.state.try_read() {
            Ok(value) => value,
            Err(_) => return Err("Could not get read lock on VM state".to_string()),
        };

        let get_result = gc.get(&arg.pointer);

        get_result.map(|value| value)
    }

    pub fn ref_count(&self, arg: &ValueReference) -> ExecResult<usize> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = match self.state.try_read() {
            Ok(value) => value,
            Err(_) => return Err("Could not get read lock on VM state".to_string()),
        };

        match gc.ref_count(arg.pointer.id) {
            Some(count) => Ok(count),
            None => Err("Could not get reference count".to_string()),
        }
    }

    /// Converts a pointer into a value reference that can be used by the VM's caller.
    /// Counts the pointer if it is uncounted.
    pub fn value_ref_from_ptr(&self, mut ptr: GCPointer<StoredData>) -> ExecResult<ValueReference> {
        // if the pointer is uncounted, we need to manually count it
        if !ptr.counted {
            let mut gc = match self.state.try_write() {
                Ok(value) => value,
                Err(_) => return Err("Could not get write lock on VM state".to_string()),
            };

            match gc.count_pointer(&mut ptr) {
                Ok(_) => {},
                Err(_) => return Err("Could not count pointer".to_string()),
            }
        }

        let result = ValueReference::new(ptr, &self);

        return Ok(result)
    }

    /// Stores a value in the VM's state, returning a reference to the stored value
    pub fn store_value(&self, value: StoredData) -> ExecResult<Vec<ValueReference>> {
        // try to get write lock
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        // allocate value
        let ptr = match gc.allocate(value) {
            Ok(ptr) => ptr,
            Err(msg) => return Err(format!("Could not allocate value: {}", msg).to_string()),
        };

        // create value reference
        return match self.value_ref_from_ptr(ptr) {
            Ok(value_ref) => Ok(vec![value_ref]),
            Err(msg) => Err(format!("Could not create value reference: {}", msg).to_string()),
        }
    }

    /// Drops a reference to a value from the VM's state, decrementing the reference count.
    pub fn drop_reference(&self, reference: &mut ValueReference) {
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => panic!("Could not get write lock on VM state"),
        };

        let drop_result = gc.drop_pointer(&mut reference.pointer);

        if let Err(msg) = drop_result {
            panic!("Could not drop pointer: {}", msg);
        }
    }

    /// Clones a value reference, incrementing the reference count.
    pub fn clone_reference(&self, reference: &ValueReference) -> ExecResult<ValueReference> {
        let mut cloned_ptr = reference.pointer.clone();

        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        match gc.count_pointer(&mut cloned_ptr) {
            Ok(_) => {}
            Err(msg) => return Err(msg),
        }

        let new_reference = ValueReference::new(cloned_ptr, self);

        return Ok(new_reference)
    }

    /// Fills a buffer with the given value
    fn execute_fill_buffer(&self, buffer: &ValueReference, value: StoredData) -> ExecResult<Vec<ValueReference>> {
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        let fill_result = gc.fill_buffer(&buffer.pointer, value);

        if let Err(msg) = fill_result {
            return Err(msg);
        }

        return Ok(vec![])
    }

    fn handle_op_null_result(&self, operand: StoredData, op: &str) -> ExecResult<Vec<ValueReference>> {
        let arg_type: StringLive = operand.as_live().type_code().unwrap_or_else(|_| "unknown".to_string());
        Err(format!("Cannot execute {} on type {}, operation not supported", op, arg_type))
    }

    fn handle_op_result(&self, result: ExecResult<StoredData>) -> ExecResult<Vec<ValueReference>> {
        match result {
            // If the result is a pointer, we can convert it directly to a value reference (but it needs to be counted)
            Ok(StoredData::PointerStored(ptr)) => self.value_ref_from_ptr(ptr).map(|value_ref| vec![value_ref]),
            // Otherwise, we need to store the result value and return a reference to it
            Ok(result) => self.store_value(result),
            Err(msg) => Err(msg)
        }
    }

    fn execute_as_int(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_int, StoredData::IntStored)
    }


    fn execute_as_float(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_float, StoredData::FloatStored)
    }

    fn execute_as_string(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_string, StoredData::StringStored)
    }

    fn execute_as_bool(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_bool, StoredData::BoolStored)
    }

    fn execute_as_pointer(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_pointer, StoredData::PointerStored)
    }

    fn execute_as_list(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_list, StoredData::ListStored)
    }

    fn execute_as_dict(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_cast_op!(self, arg, as_dict, StoredData::DictStored)
    }

    fn execute_add(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_add, lhs, rhs)
    }

    fn execute_sub(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_sub, lhs, rhs)
    }

    fn execute_mul(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_mul, lhs, rhs)
    }

    fn execute_div(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_div, lhs, rhs)
    }

    fn execute_mod(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_mod, lhs, rhs)
    }

    fn execute_pow(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_pow, lhs, rhs)
    }

    fn execute_if(&self, condition: &ValueReference, then: &ValueReference, otherwise: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_three_arg_op!(self, op_if, condition, then, otherwise)
    }

    fn execute_not(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_one_arg_op!(self, op_not, arg)
    }

    fn execute_and(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_and, lhs, rhs)
    }

    fn execute_or(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_or, lhs, rhs)
    }

    fn execute_equal(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_eq, lhs, rhs)
    }

    fn execute_less_than(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_lt, lhs, rhs)
    }

    fn execute_greater_than(&self, lhs: &ValueReference, rhs: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_gt, lhs, rhs)
    }

    fn execute_is_null(&self, arg: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let arg_value: StoredData = self.get_ref_value(arg).map_err(|msg| msg)?;

        arg_value.clone().as_live().is_null().map_or_else(
            || self.handle_op_null_result(arg_value, stringify!($op)),
            |result| self.handle_op_result(result.map(|value| StoredData::BoolStored(value))))
    }

    fn execute_length(&self, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let list_value: StoredData = self.get_ref_value(list).map_err(|msg| msg)?;

        list_value.clone().as_live().op_len().map_or_else(
            || {
                let arg_type: StringLive = list_value.as_live().type_code().unwrap_or_else(|_| "unknown".to_string());
                Err(format!("Cannot execute op_len on type {}, operation not supported", arg_type))
            },
            |result| {
                let result_value = result?;
                let stored_result = StoredData::IntStored(result_value);
                self.store_value(stored_result)
            }
        )
    }

    fn execute_get_item(&self, collection: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_get_item, collection, index)
    }

    fn execute_set_item(&self, collection: &ValueReference, index: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let collection_val = self.get_ref_value(collection)?;
        let index_val = self.get_ref_value(index)?;
        // The gc will automatically count the cloned pointer once we allocate the new list.
        let val_ptr = value.pointer.clone();

        collection_val.clone().as_live().op_set_item(&index_val, val_ptr).map_or_else(
            || self.handle_op_null_result(collection_val, stringify!($op)),
            |result| self.handle_op_result(result)
        )
    }

    fn execute_push(&self, list: &ValueReference, value: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let list_val = self.get_ref_value(list)?;
        // The gc will automatically count the cloned pointer once we allocate the new list.
        let val_ptr = value.pointer.clone();

        list_val.clone().as_live().op_push(val_ptr).map_or_else(
            || self.handle_op_null_result(list_val, stringify!($op)),
            |result| self.handle_op_result(result)
        )
    }

    fn execute_remove(&self, list: &ValueReference, index: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        execute_two_arg_op!(self, op_remove, list, index)
    }

    fn execute_call(&self, func: &ValueReference, args: Vec<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
        // get the function
        let func = match self.get_ref_value(func) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get function: {}", msg))
        };
        let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

        // get the args and ensure that there are the correct number of them
        let mut args_cloned: Vec<ValueReference> = Vec::new();
        for arg in args {
            args_cloned.push(self.clone_reference(arg)?);
        }

        let result = self.handle_call_function(&func, &args_cloned);

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::core::vm::store_op::StoreOp;
    use crate::core::vm::VM;
    use crate::core::data::live::{LiveData};
    use crate::core::vm::value_ref::ValueReference;

    #[test]
    fn test_store_pointer() {
        let mut vm = VM::new(4);

        // test storing pointers
        test_store_pointer_helper(&mut vm, "hello");
        assert_eq!(vm.object_count(), 0);

        test_store_pointer_helper(&mut vm, "");
        assert_eq!(vm.object_count(), 0);

    }

    fn test_store_pointer_helper2<'a>(vm: &'a mut VM, value: &str) -> ValueReference<'a> {
        let st_results: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(value.to_string())).unwrap();
        let ptr = st_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 1);

        let st_results_meta: Vec<ValueReference> = vm.execute_store(StoreOp::StorePointer(ptr)).unwrap();
        let meta_ptr = st_results_meta.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 2);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        println!("{:?}", vm.state);
        //
        // println!("{:?}", meta_ptr.get().unwrap().as_live().as_pointer().unwrap());

        assert_eq!(vm.get_ref_value(meta_ptr).unwrap().as_live().as_pointer(), Some(Ok(ptr.pointer.clone())));

        // There should be 2 objects in the VM: the string and the pointer
        assert_eq!(vm.object_count(), 2);

        // println!("{:?}", vm.state);

        let result = ptr.clone();

        // there should now be three references to the pointer: ptr, the pointer in the meta, and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 3);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        drop(st_results_meta);

        // there should now be two references to the pointer: ptr and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 2);

        drop(st_results);

        // there should now be one reference to the pointer: the result
        assert_eq!(vm.ref_count(&result).unwrap(), 1);

        return result;
    }

    fn test_store_pointer_helper(vm: &mut VM, value: &str) {
        vm.reset();

        let ptr = test_store_pointer_helper2(vm, value);

        // println!("{:?}", ptr);

        assert_eq!(ptr.vm.ref_count(&ptr).unwrap(), 1);

        // There should be 1 object in the VM: the pointer
        assert_eq!(ptr.vm.object_count(), 1);
    }





}