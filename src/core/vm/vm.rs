use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::core::data::live::{FuncLive, FuncOpLive, FuncValLive, LiveData, PointerLive, StringLive};
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
            StoreOp::StoreFunctionVal(_, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionOp(_, _, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionGraph(func) => self.store_function(&func),
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

    fn execute_call(&self, func: &ValueReference, args: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        // get the function
        let func = match self.get_ref_value(func) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get function: {}", msg))
        };
        let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

        // get the args list
        let args_val = match self.get_ref_value(args) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get args: {}", msg))
        };
        let args_list = args_val.as_live().as_list().ok_or_else(|| "Cannot call a function with a non-list value as arguments".to_string())??;

        // get the args as value references
        let mut args: Vec<ValueReference> = Vec::new();
        for arg_ptr in args_list {
            let arg_val = self.value_ref_from_ptr(arg_ptr)?;
            args.push(arg_val);
        }

        let result = self.handle_call_function(&func, &args);

        result
    }

    /// Handles the call of an operation that is part of a function.
    pub fn handle_call_function_op(&self, func_op: &FuncOpLive, context: &HashMap<String, ValueReference>) -> ExecResult<Vec<ValueReference>> {
        // get each arg value from the context
        let mut arg_values: Vec<&ValueReference> = Vec::new();
        for arg_ptr in func_op.input_vals.iter() {
            // get the stored data at the pointer
            let get_result = self.state.read().unwrap().get(arg_ptr)?;

            // convert it to a func value
            let func_val = match get_result {
                StoredData::FuncValStored(func_val) => func_val,
                _ => return Err("Function operation cannot take a non-func value as an argument".to_string())
            };
            
            // get the ref from the context by looking up the func val's guid
            let arg_ref = context.get(&func_val.guid).ok_or_else(|| "Function operation cannot find argument value in context".to_string())?;
            
            // add the ref to the arg values
            arg_values.push(arg_ref);
        }
        
        // get an operation using the opcode and arg values
        let op = func_op.opcode.to_operation(&arg_values);
        
        // execute the operation and return its result
        self.execute_op(op)
    }

    /// Handles the call of a function.
    pub fn handle_call_function<'a>(&'a self, func: &FuncLive, args: &Vec<ValueReference<'a>>) -> ExecResult<Vec<ValueReference<'a>>> {
        // make sure the function has the right number of args
        if func.input_vals.len() != args.len() {
            return Err(format!("Function expected {} arguments, but got {}", func.input_vals.len(), args.len()));
        }

        // create a new context for the function for storing known values
        let mut context: HashMap<String, ValueReference> = HashMap::new();

        // create a queue to hold the operations that need to be executed
        let mut op_queue: Vec<FuncOpLive> = Vec::new();

        // bind the args to the context
        for (i, arg) in args.iter().enumerate() {
            let input_ptr: &PointerLive = func.input_vals.get(i).unwrap();
            let input_val: StoredData = match self.state.read().unwrap().get(input_ptr) {
                Ok(val) => val,
                Err(msg) => return Err(format!("Function cannot find function input value (index: {}): {}", i, msg))
            };
            let input_val_live: FuncValLive = match input_val.as_live().as_func_val() {
                Some(Ok(val)) => val,
                Some(Err(msg)) => return Err(format!("Function cannot take a non-func value as an argument: {}", msg)),
                None => return Err("Function cannot take a non-func value as an argument".to_string())
            };
            context.insert(input_val_live.guid.clone(), arg.clone());

            // get the dependent operations on the arg and add them to the queue
            let dependent_ops: Vec<PointerLive> = input_val_live.dependents.clone();
            for dependent_op in dependent_ops {
                let dependent_op_val = match self.state.read().unwrap().get(&dependent_op) {
                    Ok(val) => val,
                    Err(msg) => return Err(format!("Function cannot find dependent operation: {}", msg))
                };
                let dependent_op_val_live: FuncOpLive = match dependent_op_val.as_live().as_func_op() {
                    Some(Ok(val)) => val,
                    Some(Err(msg)) => return Err(format!("Function cannot execute a non-func-op value: {}", msg)),
                    None => return Err("Function cannot execute a non-func-op value".to_string())
                };
                op_queue.push(dependent_op_val_live);
            }
        }

        // execute each operation in the queue until it is empty
        while !op_queue.is_empty() {
            // get the op that was added first
            let op = op_queue.remove(0);

            // check if all of the op's inputs are in the context
            let mut all_inputs_known = true;
            for (arg_index, input_ptr) in op.input_vals.iter().enumerate() {
                let input_val: StoredData = match self.state.read().unwrap().get(input_ptr) {
                    Ok(val) => val,
                    Err(msg) => return Err(format!("Cannot find input value (index: {}) for operation {}: {}", arg_index, op.opcode, msg))
                };
                let input_val_live: FuncValLive = match input_val.as_live().as_func_val() {
                    Some(Ok(val)) => val,
                    Some(Err(msg)) => return Err(format!("Cannot take a non-func value as an argument for operation {}: {}", op.opcode, msg)),
                    None => return Err("Cannot take a non-func value as an argument".to_string())
                };

                // if the val is a constant but is not in the context, add it to the context
                if let Some(constant_ptr) = input_val_live.constant {
                    if !context.contains_key(&input_val_live.guid) {
                        let constant_val_ref: ValueReference = self.value_ref_from_ptr(constant_ptr.clone())?;
                        context.insert(input_val_live.guid.clone(), constant_val_ref);
                    }
                }

                if !context.contains_key(&input_val_live.guid) {
                    all_inputs_known = false;
                    break;
                }
            }

            // if not all inputs are known, execution should be skipped.
            // once the final missing input value is known, the operation will be added to the queue again and executed.
            if !all_inputs_known {
                continue;
            }

            // if all inputs are known, execute the operation
            let result_val_refs: Vec<ValueReference> = match self.handle_call_function_op(&op, &context) {
                Ok(val_refs) => val_refs,
                Err(msg) => return Err(format!("Execution of operation {} failed: {}", op.opcode, msg))
            };

            // make sure the number of result vals matches the number of output vals
            let num_outputs: usize = match op {
                _ => 1  // since functions can only call operations at the moment, and all operations have only one output
            };
            if result_val_refs.len() != num_outputs {
                return Err(format!("Function operation expected {} result values, but got {}", num_outputs, result_val_refs.len()));
            }

            let result_func_val_ptrs: Vec<&PointerLive> = match op {
                _ => vec![&op.output_val]  // ops can only have one output
            };

            for (i, result_val_ref) in result_val_refs.iter().enumerate() {
                // get the func val associated with this output
                let output_ptr: &PointerLive = result_func_val_ptrs.get(i).unwrap();
                let output_val: StoredData = match self.state.read().unwrap().get(&output_ptr) {
                    Ok(val) => val,
                    Err(msg) => return Err(format!("Cannot find output value {} for operation {}: {}", i, op.opcode, msg))
                };
                let output_func_val: FuncValLive = output_val.as_live().as_func_val().ok_or_else(|| "Function operation cannot return a non-func value".to_string())??;

                // add the result value to the context
                context.insert(output_func_val.guid.clone(), result_val_ref.clone());

                // add the dependent operations on the result value to the queue
                for dependent_op_ptr in output_func_val.dependents.iter() {
                    let dependent_op_val: StoredData = match self.state.read().unwrap().get(&dependent_op_ptr) {
                        Ok(val) => val,
                        Err(msg) => return Err(format!("Cannot find dependent operation (pointer id: {}) for operation {}: {}", dependent_op_ptr.id, op.opcode, msg))
                    };
                    let dependent_op_val_live: FuncOpLive = dependent_op_val.as_live().as_func_op().ok_or_else(|| "Function cannot execute a non-func-op value".to_string())??;
                    op_queue.push(dependent_op_val_live);
                }
            }
        }

        // get the return values from the context
        let mut return_values: Vec<ValueReference> = Vec::new();

        for output_ptr in func.output_vals.iter() {
            let output_val: StoredData = match self.state.read().unwrap().get(output_ptr) {
                Ok(val) => val,
                Err(msg) => return Err(format!("Cannot find output value (pointer id: {}) for function: {}", output_ptr.id, msg))
            };
            let output_val_live: FuncValLive = output_val.as_live().as_func_val().ok_or_else(|| "Function cannot return a non-func value".to_string())??;
            let return_value: ValueReference = context.get(&output_val_live.guid).ok_or_else(|| "Function cannot find return value in context".to_string())?.clone();
            return_values.push(return_value);
        }

        Ok(return_values)
    }

    /// Applies a function to each item in a list, returning a new list of the results.
    pub fn map(&self, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        // get the function
        let func = match self.get_ref_value(func) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get function: {}", msg))
        };
        let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

        // get the list
        let list_val = match self.get_ref_value(list) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get list: {}", msg))
        };
        let list_val = list_val.as_live().as_list().ok_or_else(|| "Cannot call a function with a non-list value as arguments".to_string())??;

        // create a new list to hold the results
        let mut result_list: Vec<ValueReference> = Vec::new();

        // call the function on each item in the list
        for item_ptr in list_val {
            let item_val_ref = self.value_ref_from_ptr(item_ptr.clone())?;
            let result_val = self.handle_call_function(&func, &vec![item_val_ref])?;
            result_list.push(result_val[0].clone());
        }

        // store the result list
        let store_list_result = self.store_value(StoredData::ListStored(result_list.iter().map(|val_ref| val_ref.pointer.clone()).collect()))?;
        Ok(store_list_result)
    }

    /// Wrapper function to handle lifetime issues with calling reduce.
    pub fn handle_reduce(&self, func: &ValueReference, list: &ValueReference, initial: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        // create a new reference to the initial value to avoid lifetime issues
        let initial = self.value_ref_from_ptr(initial.pointer.clone())?;

        self.reduce(func, list, &initial)
    }

    // applies a combining function to each item in a list, returning a single result.
    pub fn reduce<'a>(&'a self, func: &ValueReference, list: &ValueReference, initial: &ValueReference<'a>) -> ExecResult<Vec<ValueReference<'a>>> {
        // get the function
        let func = match self.get_ref_value(func) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get function: {}", msg))
        };
        let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

        // get the list
        let list_val = match self.get_ref_value(list) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get list: {}", msg))
        };
        let list_val = list_val.as_live().as_list().ok_or_else(|| "Cannot call a function with a non-list value as arguments".to_string())??;

        let mut last_result = initial.clone();

        for item_ptr in list_val {
            let item_val_ref = self.value_ref_from_ptr(item_ptr.clone())?;
            let result_val = self.handle_call_function(&func, &vec![last_result, item_val_ref])?;
            last_result = result_val[0].clone();
        }

        Ok(vec![last_result])
    }

    // gets the items in a list that match a given condition
    pub fn filter(&self, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        // get the function
        let func = match self.get_ref_value(func) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get function: {}", msg))
        };
        let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

        // get the list
        let list_val = match self.get_ref_value(list) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Failed to get list: {}", msg))
        };
        let list_val = list_val.as_live().as_list().ok_or_else(|| "Cannot call a function with a non-list value as arguments".to_string())??;

        // create a new list to hold the results
        let mut result_list: Vec<ValueReference> = Vec::new();

        // call the function on each item in the list
        for item_ptr in list_val {
            let item_val_ref = self.value_ref_from_ptr(item_ptr.clone())?;
            let result_val = self.handle_call_function(&func, &vec![item_val_ref.clone()])?;
            let result_val_ref = result_val[0].clone();
            let result_val = self.get_ref_value(&result_val_ref)?;
            let result_val = result_val.as_live().as_bool().ok_or_else(|| "Cannot filter a list with a non-bool function".to_string())??;
            if result_val {
                result_list.push(item_val_ref);
            }
        }

        // store the result list
        let store_list_result = self.store_value(StoredData::ListStored(result_list.iter().map(|val_ref| val_ref.pointer.clone()).collect()))?;
        Ok(store_list_result)
    }
}