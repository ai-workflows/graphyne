use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::sync::mpsc::{channel, Sender};
use crossbeam::queue::SegQueue;
use crate::core::data::live::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult, Symbol};
use crate::core::data::functions::OpCode;
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    /// Helper function to get a function value/variable from a pointer.
    fn get_func_val(&self, ptr: &PointerLive) -> ExecResult<FuncValLive> {
        let val = match self.state.read().unwrap().get(ptr) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Cannot find func val (pointer id: {}) for function: {}", ptr.id, msg))
        };
        let func_val = match val {
            StoredData::FuncValStored(func_val) => func_val,
            _ => {
                return match self.get_stored_type(&val) {
                    Ok(val_type) => Err(format!("Expected func val but got type: {}", val_type.get_name())),
                    Err(msg) => Err(format!("Expected func val but got unknown type (failed to get type: {})", msg))
                };
                ;
            }
        };
        Ok(func_val)
    }

    /// Handles the call of an operation that is part of a function.
    /// Gets the arguments from the context, executes the operation, and returns the result values.
    pub fn handle_call_function_op<'a>(&'a self, func_op: &FuncOpLive, context: Arc<RwLock<HashMap<String, ValueReference<'a>>>>) -> ExecResult<Vec<ValueReference>> {

        let arg_values = self.get_func_op_args(func_op, context)?;
        let arg_values: Vec<&_> = arg_values.iter().collect();
        let op = func_op.opcode.to_operation(&arg_values);
        self.execute_op(op)
    }

    fn get_val_from_context<'a>(&self, ptr: &PointerLive, context: &RwLockReadGuard<'_, HashMap<String, ValueReference<'a>>>) -> ExecResult<ValueReference<'a>> {
        // get the read lock on the context
        let func_val = self.get_func_val(ptr)
            .map_err(|msg| format!("Function cannot get value from context: {}", msg))?;

        let val = match context.get(&func_val.guid) {
            Some(val) => val,
            None => return Err(format!("Cannot find value (pointer id: {}) in context", ptr.id))
        };
        Ok(val.clone())
    }

    /// Gets the arguments to a function's operation from the context.
    fn get_func_op_args<'a>(&'a self, func_op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>) -> ExecResult<Vec<ValueReference>> {

        let list: ExecResult<Vec<ValueReference>> = func_op.input_vals.iter()
            .map(move |arg_ptr| {
                self.get_val_from_context(arg_ptr, &context.read().unwrap())
            })
            .collect();

        list
    }

    /// Gets the operations that are dependent on a particular value that is now known, and adds them to the operation queue.
    fn handle_func_val_dependents(&self, func_val: &FuncValLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<Vec<FuncOpLive>> {
        let mut op_queue = Vec::new();

        for dependent_op in &func_val.dependents {
            let state = self.state.read().unwrap();
            let dependent_op_val_live = state.get(dependent_op)
                .map_err(|msg| format!("Function cannot find dependent operation: {}", msg))?
                .as_live().as_func_op()
                .ok_or_else(|| "Function cannot execute a non-func-op value".to_string())?
                .map_err(|msg| format!("Function cannot execute a non-func-op value: {}", msg))?;

            // get the read lock on the context
            let context = context.read().unwrap();

            // check if the operation has already been executed
            let output_ptr = dependent_op_val_live.output_vals.get(0)
                .ok_or_else(|| "Function operation has no output value".to_string())?;
            let output_val = self.get_func_val(output_ptr)
                .map_err(|msg| format!("Function cannot get output value for operation {}: {}", dependent_op_val_live.opcode, msg))?;
            if context.contains_key(&output_val.guid) {
                continue;
            }

            // check if all inputs are known
            let inputs_known = dependent_op_val_live.input_vals.iter()
                .all(|input_ptr| {
                    let input_val = self.get_func_val(input_ptr)
                        .map_err(|msg| format!("Function cannot get input value for operation {}: {}", dependent_op_val_live.opcode, msg));

                    let input_val = match input_val {
                        Ok(input_val) => input_val,
                        Err(_) => return false
                    };

                    context.contains_key(&input_val.guid)
                });

            if !inputs_known {
                continue;
            }

            // add the operation to the queue
            op_queue.push(dependent_op_val_live.clone());
        }
        Ok(op_queue)
    }

    /// Initializes the call of a function by binding the arguments and constants to the context.
    fn initialize_func_call<'a>(&'a self,
                                func: &FuncLive,
                                args: &[ValueReference<'a>],
                                context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>,
                                op_queue: Arc<SegQueue<FuncOpLive>>
    ) -> ExecResult<()> {
        self.bind_args_to_context(func, args, context.clone(), op_queue.clone())?;
        self.handle_constants(func, context, op_queue)?;
        Ok(())
    }

    /// Takes the arguments to a function as a list of value references, gets their corresponding func value nodes, and binds them to the context.
    fn bind_args_to_context<'a>(&'a self,
                                func: &FuncLive,
                                args: &[ValueReference<'a>],
                                context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>,
                                op_queue: Arc<SegQueue<FuncOpLive>>
    ) -> ExecResult<()> {
        for (i, arg_value) in args.iter().enumerate() {
            let input_ptr = func.input_vals.get(i)
                .ok_or("Function input value missing")?;
            let input = self.get_func_val(input_ptr)
                .map_err(|msg| format!("Function cannot get input value: {}", msg))?;

            context.write().unwrap().insert(input.guid.clone(), arg_value.clone());
            let new_ops = self.handle_func_val_dependents(&input, context.clone())?;
            for new_op in new_ops {
                op_queue.push(new_op);
            }
        }
        Ok(())
    }

    /// Handles the constant values present in a function's scope by retrieving their values and binding them to the context.
    fn handle_constants<'a>(&'a self,
                            func: &FuncLive,
                            context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>,
                            op_queue: Arc<SegQueue<FuncOpLive>>
    ) -> ExecResult<()> {
        for constant_ptr in &func.constant_vals {
            let constant_val = self.get_func_val(constant_ptr)
                .map_err(|msg| format!("Function cannot get constant value: {}", msg))?;

            let constant_ref = match constant_val.constant.as_ref() {
                Some(ptr) => self.value_ref_from_ptr(ptr.clone())?,
                None => return Err("Function expected constant but none found".to_string())
            };

            context.write().unwrap().insert(constant_val.guid.clone(), constant_ref);
            let new_ops = self.handle_func_val_dependents(&constant_val, context.clone())?;
            for new_op in new_ops {
                op_queue.push(new_op);
            }
        }
        Ok(())
    }


    /// Executes an operation within the scope of a function using the provided context.
    /// Retrieves the arg values from the context, executes the operation, and returns the result values.
    fn try_execute_fn_op<'a>(&'a self, op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>) -> ExecResult<Vec<ValueReference>> {
        self.validate_op_inputs(op, context.clone())?;
        let result_val_refs = self.handle_call_function_op(op, context.clone())
            .map_err(|msg| format!("Execution of operation {} failed: {}", op.opcode, msg))?;

        if result_val_refs.len() != op.output_vals.len() {
            return Err(format!("Function operation expected {} result values, but got {}", op.output_vals.len(), result_val_refs.len()));
        }

        // get the write lock on the context
        let mut context = context.write().unwrap();

        // loop through the outputs from the executed operation
        for (output_ptr, result_val_ref) in op.output_vals.iter().zip(&result_val_refs) {
            // get the output
            let output_val = self.get_func_val(output_ptr)
                .map_err(|msg| format!("Function cannot get output value for operation {}: {}", op.opcode, msg))?;

            // make sure the output is not already in the context
            if context.contains_key(&output_val.guid) {
                return Err(format!("Function cannot overwrite output value {:?} in context", match output_val.symbol {
                    Some(ref symbol) => symbol.clone(),
                    None => output_val.guid.clone()
                }));
            }

            // store the output in the context
            context.insert(output_val.guid.clone(), result_val_ref.clone());
        }


        Ok(result_val_refs)
    }

    /// Validates the inputs to function's operation by checking that all args are present in the context.
    fn validate_op_inputs<'a>(&'a self, op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>) -> ExecResult<()> {
        // get the read lock on the context
        let context = context.read().unwrap();

        op.input_vals.iter().enumerate().try_for_each(|(arg_index, input_ptr)| {
            let input_val = self.get_func_val(input_ptr)
                .map_err(|msg| format!("Function cannot get input value (index {}): {}", arg_index, msg))?;

            if !context.contains_key(&input_val.guid) {
                Err(format!("Operation {} cannot be executed because input value index {} is not known", op.opcode, arg_index))
            } else {
                Ok(())
            }
        })
    }

    /// Manages the call of function operations and the queuing of its dependent operations for execution.
    fn manage_op_queue<'a>(
        &'a self,
        op_queue: Arc<SegQueue<FuncOpLive>>,
        context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>
    ) -> ExecResult<()> {
        while let Some(op) = op_queue.pop() {
            let (tx, rx) = channel();
            // Spawning a new thread to execute the operation
            self.thread_pool.scope(|s| {
                let context = context.clone();
                let op = op.clone();

                s.spawn(move |_| {
                    let result = self.try_execute_fn_op(&op, context.clone());
                    // Sending the result back to the main thread
                    tx.send(result).expect("Failed to send result");
                });
            });

            // Processing the result to queue dependent operations
            if let Ok(result) = rx.recv() {
                match result {
                    Ok(_) => (),
                    Err(e) => return Err(format!("Error executing operation {}: {}", op.opcode, e))
                }

                for output_ptr in &op.output_vals {
                    let output_val = self.get_func_val(output_ptr)
                        .map_err(|msg| format!("Function cannot get output value for operation {}: {}", op.opcode, msg))?;

                    let new_ops = self.handle_func_val_dependents(&output_val, context.clone())
                        .map_err(|msg| format!("Function cannot handle dependents for operation {}: {}", op.opcode, msg))?;

                    for new_op in new_ops {
                        op_queue.push(new_op);
                    }
                }
            }
        }

        Ok(())
    }

    fn get_func_call_outputs<'a>(&'a self, func: &FuncLive, context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>) -> ExecResult<Vec<ValueReference<'a>>> {
        // obtain the read lock on the context
        let context = context.read().unwrap();

        func.output_vals.iter()
            .map(|output_ptr| {
                let output_val = self.get_func_val(output_ptr)
                    .map_err(|msg| format!("Function cannot get output value for function: {}", msg))?;
                context.get(&output_val.guid)
                    .cloned()
                    .ok_or_else(|| format!("Function cannot find output value {:?} in context", match output_val.symbol {
                        Some(ref symbol) => symbol.clone(),
                        None => output_val.guid.clone()
                    }))
            })
            .collect()
    }

    fn validate_func_inputs(&self, func: &FuncLive, args: &[ValueReference]) -> ExecResult<()> {
        // Check if the function has the right number of args
        if func.input_vals.len() != args.len() {
            return Err(format!("Function expected {} arguments, but got {}", func.input_vals.len(), args.len()));
        }
        Ok(())
    }

    /// Handles the call of a function.
    pub fn handle_call_function<'a>(&'a self, func: &FuncLive, args: &[ValueReference<'a>]) -> ExecResult<Vec<ValueReference<'a>>> {
        match self.validate_func_inputs(func, args) {
            Ok(_) => (),
            Err(msg) => return Err(format!("Function call failed: {}", msg))
        }

        let context: Arc<RwLock<HashMap<Symbol, ValueReference>>> = Arc::new(RwLock::new(HashMap::new()));
        let op_queue: Arc<SegQueue<FuncOpLive>> = Arc::new(SegQueue::new());

        // Initialize the function call
        self.initialize_func_call(func, args, context.clone(), op_queue.clone())?;

        // Execute the function's operations
        self.manage_op_queue(op_queue, context.clone())?;

        self.get_func_call_outputs(func, context)
    }

    pub fn execute_call(&self, func: &ValueReference, args: Vec<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
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

    // Version of try_execute_fn_op that returns result values by calling a callback function as soon as they are known.
    fn try_execute_fn_op_async<'a, F>(&'a self, op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference<'a>>>>, mut return_callback: Box<fn(usize, ValueReference) -> ExecResult<()>>) -> ExecResult<()>
    {
        // if this is not a function call, execute the operation normally
        if op.opcode != OpCode::Call {
            let execute_result = self.try_execute_fn_op(op, context);
            let results = match execute_result {
                Ok(results) => results,
                Err(e) => return Err(format!("Error executing operation {}: {}", op.opcode, e))
            };

            for (i, result) in results.iter().enumerate() {
                return_callback(i, result.clone())?;
            }

            return Ok(());
        }

        // otherwise, return the results as they come in

        // validate the call operation's inputs
        self.validate_op_inputs(op, context.clone())?;

        let arg_values: Vec<ValueReference> = self.get_func_op_args(op, context)?;
        let arg_values: Vec<&ValueReference> = arg_values.iter().collect();
        let func_ref = arg_values[0];
        let args: Vec<&ValueReference> = arg_values[1..].to_vec();

        // call the function
        self.execute_call_async(func_ref, args, return_callback)
    }

    /// Handles a function call with a callback that is executed when a value is returned.
    pub fn handle_call_function_async<'a>(&'a self, func: &FuncLive, args: &[ValueReference<'a>], mut return_callback: Box<fn(usize, ValueReference) -> ExecResult<()>>) -> ExecResult<()>
    {
        // validate the function's inputs
        match self.validate_func_inputs(func, args) {
            Ok(_) => (),
            Err(msg) => return Err(format!("Function call failed: {}", msg))
        }

        // Get a set of the function's output symbols
        let output_symbols: HashMap<Symbol, usize> = func.output_vals.iter()
            .enumerate()
            .map(|(i, output_ptr)| {
                let output_val = self.get_func_val(output_ptr)
                    .map_err(|msg| format!("Function cannot get output value for function: {}", msg))?;
                Ok((output_val.guid.clone(), i))
            })
            .collect::<ExecResult<_>>()?;

        // Context is a map of the function's values (by their symbol) to their values
        let context: Arc<RwLock<HashMap<Symbol, ValueReference>>> = Arc::new(RwLock::new(HashMap::new()));

        // Queue of operations to execute
        let op_queue: Arc<SegQueue<FuncOpLive>> = Arc::new(SegQueue::new());

        // Initialize the function call by binding the arguments and constants to the context
        // and adding their dependent operations to the queue
        self.initialize_func_call(func, args, context.clone(), op_queue.clone())?;

        // Execute the function's operations
        while let Some(op) = op_queue.pop() {
            let channel = channel();
            let tx: Sender<Box<ExecResult<usize>>> = channel.0;
            let rx = channel.1;

            // let callback = |i: usize, result: ValueReference| -> ExecResult<()> {
            //     let ptr = self.counted_ptr_from_value_ref(result)?;
            //
            //     tx.send(Ok(i)).expect("Failed to send result");
            //     Ok(())
            // };

            let boxed_callback = Box::new(move |i: usize, result: ValueReference| -> ExecResult<()> {
                let ptr = self.counted_ptr_from_value_ref(result)?;

                let res: Box<ExecResult<usize>> = Box::new(Ok((i)));


                // tx.send(res).expect("Failed to send result");
                Ok(())
            });

            // let error_callback = |msg: String| -> ExecResult<()> {
            //     tx.send(Err(msg)).expect("Failed to send result");
            //     Ok(())
            // };

            // Spawning a new thread to execute the operation
            self.thread_pool.scope(|s| {
                let context = context.clone();
                let op = op.clone();

                s.spawn(move |_| {
                    match self.try_execute_fn_op_async(&op, context.clone(), boxed_callback) {
                        Ok(_) => (),
                        // Err(e) => {
                        //     error_callback(e).expect("Failed to send error");
                        // }
                        _ => ()
                    }
                });
            });

            // Processing the result to queue dependent operations
            if let Ok(result) = rx.recv() {
                // let (i, ptr) = match result {
                //     Ok((i, ptr)) => (i, ptr),
                //     Err(e) => return Err(format!("Error executing operation {}: {}", op.opcode, e))
                // };
                //
                // // convert the ptr back to a value ref
                // let result_val_ref = self.value_ref_from_ptr(ptr)?;

                // unbox

                let i = match *result {
                    Ok(i) => i,
                    Err(e) => return Err(format!("Error executing operation {}: {}", op.opcode, e))
                };

                let output_ptr = op.output_vals.get(i)
                    .ok_or_else(|| format!("Operation {} has no output value at index {}", op.opcode, i))?;

                // get the func val for the output and add its dependents to the queue
                let output_val = self.get_func_val(output_ptr)
                    .map_err(|msg| format!("Function cannot get output value for operation {}: {}", op.opcode, msg))?;

                let new_ops = self.handle_func_val_dependents(&output_val, context.clone())
                    .map_err(|msg| format!("Function cannot handle dependents for operation {}: {}", op.opcode, msg))?;

                for new_op in new_ops {
                    op_queue.push(new_op);
                }

                // call the return callback
                return_callback(i, self.value_ref_from_ptr(output_ptr.clone())?)?;
            }
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::api::collections::collection::Collection;
    use crate::api::GraphiteApi;
    use crate::api::interface::VmInterface;
    use crate::core::data::live::IntLive;
    use crate::core::vm::VM;
    use crate::core::data::live::live_data::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
    use crate::core::ExecResult;
    use crate::core::vm::value_ref::ValueReference;

    // tests calling a function and receiving results asynchronously
    #[test]
    fn test_call_async<'a>() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

            let json_collection = r#"{
                "constants": {},
                "functions": {
                    "main": {
                        "graph": {
                            "values": [
                                "initial",
                                "a",
                                "b",
                                "c",
                                "d",
                                ["factor", 2]
                            ],
                            "ops": [
                                ["Add", ["c", "factor"], "a"],
                                ["Add", ["d", "factor"], "b"],
                                ["Add", ["b", "factor"], "c"],
                                ["Mul", ["initial", "factor"], "d"]
                            ],
                            "input_vals": ["initial"],
                            "output_vals": ["a", "b", "c", "d"]
                        }
                    }
                },
                "collections": {},
                "imports": {}
            }"#;

            let collection: Collection = match serde_json::from_str(json_collection) {
                Ok(collection) => collection,
                Err(e) => {
                    println!("{}", e);
                    panic!();
                }
            };

            api.store_collection(collection, "my_collection".to_string()).unwrap();
            let main_func_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();
            let main_func = vm.get_ref_value(&main_func_ref).unwrap().as_live().as_func().unwrap().ok().unwrap();
            drop(main_func_ref);
            let initial: IntLive = 5.into();
            api.store_int(initial, "initial".to_string()).unwrap();
            let initial_ref = api.get_path(vec!["initial".into()]).unwrap();

            // results should be calculated in the order of d, b, c, a
            let expected_order =vec![(3, 10), (1, 12), (2, 14), (0, 16)];

            let mut results: Vec<IntLive> = Vec::new();

            let callback = |i: usize, result: ValueReference| -> ExecResult<()> {
                let result = vm.get_ref_value(&result).unwrap().as_live().as_int().unwrap().ok().unwrap();
                results.push(result);
                Ok(())
            };

            vm.handle_call_function_async(&main_func, &[initial_ref], callback).unwrap();

            assert_eq!(results.len(), 4);
            for (i, result) in results.iter().enumerate() {
                let expected = expected_order[i].1;
                assert_eq!(*result, expected);
            }
        }

        assert_eq!(vm.object_count(), 0);
    }
}