use std::collections::{HashMap, HashSet};
use crate::api::functions::FunctionGraph;
use crate::core::{ExecResult, Symbol};
use crate::core::data::live::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::vm::ops::Operation;
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    /// Stores a function in the VM given its graph representation.
    /// func: The function graph to store.
    /// class_context: A reference to the class (as a dict) that the func belongs to (if any).
    pub fn store_function(&self, func: &FunctionGraph, class_context: Option<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
        // create a hashmap to store a reference to the buffer for each value symbol
        let mut values: HashMap<Symbol, ValueReference> = HashMap::new();

        // list of the stored const value for value nodes that are constants
        let mut constants: HashMap<Symbol, ValueReference> = HashMap::new();

        // list of func vals that are constants
        // we need this list since ops pointers to inputs are not counted so we need to store the constants separately.
        let mut constant_vals: Vec<ValueReference> = Vec::new();

        // create buffers for each value node
        for val in &func.values {
            // make sure that the symbol for this value is not already in the values hashmap
            if values.contains_key(&val.symbol) {
                return Err(format!("Symbol {} already exists, ensure that all symbols are unique.", val.symbol));
            }

            // create the buffer and store it in the values hashmap
            let val_refs = self.execute_store(StoreOp::CreateBuffer)?;
            let buf = val_refs[0].clone();
            values.insert(val.symbol.clone(), buf.clone());

            // if the value is a constant, store its value in memory and add it to the constants hashmap
            if let Some(constant) = &val.constant {
                let const_ref: ValueReference = match constant {
                    StoredData::PointerStored(ptr) => {
                        // if the constant is a pointer, we can just just use it as a reference
                        self.value_ref_from_ptr(ptr.clone())?
                    }
                    _ => {
                        // if the constant is not a pointer, we need to store its data in memory
                        let const_ref = self.store_value(constant.clone())?;
                        const_ref[0].clone()
                    }
                };

                constants.insert(val.symbol.clone(), const_ref);
                constant_vals.push(buf.clone());
            }
        }

        // if a class context was provided, store a buffer for the func val that will represent the self reference
        if let Some(class_context) = class_context {
            // create the buffer
            let buf = self.execute_store(StoreOp::CreateBuffer)?;
            let buf = buf[0].clone();

            // store it in the values hashmap with the symbol "self"
            if values.contains_key(&Symbol::from("self")) {
                return Err("Symbol self already exists, ensure that all symbols are unique.".to_string());
            }
            values.insert(Symbol::from("self"), buf.clone());

            // represent the self reference as a constant. This way it has a pointer to the class.
            constants.insert(Symbol::from("self"), class_context.clone());
            constant_vals.push(buf.clone());
        }

        // create a hashmap to track the ops that are dependent on each value
        let mut value_deps_helper: HashMap<Symbol, Vec<usize>> = HashMap::new();

        // create each op
        let mut ops: Vec<ValueReference> = Vec::new();

        for op in &func.ops {
            // get the input values for this op
            let mut input_val_refs: Vec<&ValueReference> = Vec::new();

            for val_id in &op.input_vals {
                let val = match values.get(&val_id.to_string()) {
                    Some(val) => val,
                    None => return Err(format!("Input value {} for op {} does not exist. Ensure that the value is defined.", val_id, op.opcode)),
                };

                input_val_refs.push(val);
            }

            // get the output value for this op
            let output_val_ref: &ValueReference = match values.get(&op.output_val) {
                Some(val) => val,
                None => return Err(format!("Output value {} for op {} does not exist. Ensure that the value is defined.", op.output_val, op.opcode)),
            };

            let store_op = StoreOp::StoreFunctionOp(op.opcode, input_val_refs, output_val_ref);
            let op_refs: Vec<ValueReference> = self.execute_store(store_op)?;  // TODO: input and output refs are not being counted
            let op_ref: ValueReference = op_refs[0].clone();
            ops.push(op_ref);

            // add to the value deps hashmap
            for val_id in &op.input_vals {
                let val_deps = value_deps_helper.entry(val_id.clone()).or_insert(Vec::new());
                val_deps.push(ops.len() - 1);
            }
        }

        // finalize the value deps hashmap
        let mut value_deps: HashMap<Symbol, Vec<&ValueReference>> = HashMap::new();

        for (val_id, op_ids) in value_deps_helper.iter() {
            let mut val_deps: Vec<&ValueReference> = Vec::new();

            for op_id in op_ids {
                let op_ref = &ops[*op_id];
                val_deps.push(op_ref);
            }

            value_deps.insert(val_id.clone(), val_deps);
        }

        // fill the buffers for each value, including the dependent ops
        for (val_id, val_ref) in values.iter() {
            let val_deps: &Vec<&ValueReference>;
            let empty_vec: Vec<&ValueReference>;
            if !value_deps.contains_key(&val_id.to_string()) {
                empty_vec = Vec::new();
                val_deps = &empty_vec;
            }
            else {
                val_deps = value_deps.get(&val_id.to_string()).unwrap();
            }

            // check if the value is a constant and get the pointer to its value if it is
            let const_ref: Option<&ValueReference> = match constants.get(&val_id.to_string()) {
                Some(const_ref) => Some(const_ref),
                None => None,
            };

            // check if the val_id is "self" and the class context is not None
            let is_self: bool = val_id.to_string() == "self" && class_context.is_some();

            // use the deps to store the func val in memory
            let store_op = StoreOp::StoreFunctionVal(val_deps.clone(), const_ref, is_self);
            let stored_val_refs = self.execute_store(store_op)?;
            let stored_val_ref = stored_val_refs[0].clone();

            // get the value of the func val that was just stored
            let val_stored = self.get_ref_value(&stored_val_ref)?;

            // fill the buffer with the value
            let fill_buf_op = Operation::SetBuffer(&val_ref, val_stored);
            self.execute_op(fill_buf_op)?;
        }

        // get the refs to the input and output values
        let mut input_val_refs: Vec<&ValueReference> = Vec::new();
        let mut output_val_refs: Vec<&ValueReference> = Vec::new();

        for val_id in &func.input_vals {
            let val_ref = values.get(&val_id.to_string()).unwrap();
            input_val_refs.push(val_ref);
        }

        for val_id in &func.output_vals {
            let val_ref = values.get(&val_id.to_string()).unwrap();
            output_val_refs.push(val_ref);
        }

        // convert the constant refs to a vector
        let constant_refs: Vec<&ValueReference> = constant_vals.iter().collect();

        // store the function in memory
        let store_func_op = StoreOp::StoreFunction(input_val_refs, output_val_refs, constant_refs);
        let stored_func_refs = self.execute_store(store_func_op)?;
        let stored_func_ref = stored_func_refs[0].clone();

        // return the reference to the function
        Ok(vec![stored_func_ref.clone()])
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
        let mut context: HashMap<Symbol, ValueReference> = HashMap::new();

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

        // handle constants
        for constant_ptr in func.constant_vals.iter() {
            // get the constant value from memory using its pointer and add it to the context
            let constant_val: StoredData = match self.state.read().unwrap().get(constant_ptr) {
                Ok(val) => val,
                Err(msg) => return Err(format!("Function cannot find constant value: {}", msg))
            };
            let constant_val_live: FuncValLive = match constant_val.as_live().as_func_val() {
                Some(Ok(val)) => val,
                Some(Err(msg)) => return Err(format!("Function cannot take a non-func value as an argument: {}", msg)),
                None => return Err("Function cannot take a non-func value as an argument".to_string())
            };

            let constant_ref: ValueReference = match constant_val_live.constant {
                Some(ptr) => self.value_ref_from_ptr(ptr.clone())?,
                None => return Err("Function cannot take a non-constant value as an argument".to_string())
            };
            context.insert(constant_val_live.guid.clone(), constant_ref);

            // get the dependent operations on the constant and add them to the queue
            let dependent_ops: Vec<PointerLive> = constant_val_live.dependents.clone();
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

        // create a set to track which ops have been executed
        let mut executed: HashSet<String> = HashSet::new();

        // execute each operation in the queue until it is empty
        while !op_queue.is_empty() {
            // get the op that was added first
            let op = op_queue.remove(0);

            // make sure the op has not already been executed
            if executed.contains(&op.guid) {
                continue;
            }

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

            // add the op to the executed set
            executed.insert(op.guid.clone());
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