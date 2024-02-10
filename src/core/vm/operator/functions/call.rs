use std::collections::{HashMap};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::sync::mpsc::{channel};
use crossbeam::queue::SegQueue;
use crate::core::data::live::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult, Symbol};
use crate::core::vm::mmu::mmu::{clone_reference, get_stored_type, MMU, value_ref_from_ptr};
use crate::core::vm::operator::operator::execute_op;
use crate::core::vm::value_ref::ValueReference;

/// Helper function to get a function value/variable from a pointer.
fn get_func_val(mmu: Arc<MMU>, ptr: &PointerLive) -> ExecResult<FuncValLive> {
    let val = match mmu.state.read().unwrap().get(ptr) {
        Ok(val) => val,
        Err(msg) => return Err(format!("Cannot find func val (pointer id: {}) for function: {}", ptr.id, msg))
    };
    let func_val = match val {
        StoredData::FuncValStored(func_val) => func_val,
        _ => {
            return match get_stored_type(mmu, &val) {
                Ok(val_type) => Err(format!("Expected func val but got type: {}", val_type.get_name())),
                Err(msg) => Err(format!("Expected func val but got unknown type (failed to get type: {})", msg))
            };
        }
    };
    Ok(func_val)
}

/// Handles the call of an operation that is part of a function.
/// Gets the arguments from the context, executes the operation, and returns the result values.
pub fn handle_call_function_op(mmu: Arc<MMU>, func_op: &FuncOpLive, context: Arc<RwLock<HashMap<String, ValueReference>>>) -> ExecResult<Vec<ValueReference>> {

    let arg_values = get_func_op_args(mmu.clone(), func_op, context)?;
    let arg_values: Vec<&_> = arg_values.iter().collect();
    let op = func_op.opcode.to_operation(&arg_values);
    execute_op(mmu, op)
}

fn get_val_from_context(mmu: Arc<MMU>, ptr: &PointerLive, context: &RwLockReadGuard<'_, HashMap<String, ValueReference>>) -> ExecResult<ValueReference> {
    // get the read lock on the context
    let func_val = get_func_val(mmu, ptr)
        .map_err(|msg| format!("Function cannot get value from context: {}", msg))?;

    let val = match context.get(&func_val.guid) {
        Some(val) => val,
        None => return Err(format!("Cannot find value (pointer id: {}) in context", ptr.id))
    };
    Ok(val.clone())
}

/// Gets the arguments to a function's operation from the context.
fn get_func_op_args(mmu: Arc<MMU>, func_op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<Vec<ValueReference>> {

    let list: ExecResult<Vec<ValueReference>> = func_op.input_vals.iter()
        .map(move |arg_ptr| {
            get_val_from_context(mmu.clone(), arg_ptr, &context.read().unwrap())
        })
        .collect();

    list
}

/// Gets the operations that are dependent on a particular value that is now known, and adds them to the operation queue.
fn handle_func_val_dependents(mmu: Arc<MMU>, func_val: &FuncValLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<Vec<FuncOpLive>> {
    let mut op_queue = Vec::new();

    for dependent_op in &func_val.dependents {
        let state = mmu.state.read().unwrap();
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
        let output_val = get_func_val(mmu.clone(), output_ptr)
            .map_err(|msg| format!("Function cannot get output value for operation {}: {}", dependent_op_val_live.opcode, msg))?;
        if context.contains_key(&output_val.guid) {
            continue;
        }

        // check if all inputs are known
        let inputs_known = dependent_op_val_live.input_vals.iter()
            .all(|input_ptr| {
                let input_val = get_func_val(mmu.clone(), input_ptr)
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
fn initialize_func_call(mmu: Arc<MMU>,
                        func: &FuncLive,
                        args: &[ValueReference],
                        context: Arc<RwLock<HashMap<Symbol, ValueReference>>>,
                        op_queue: Arc<SegQueue<FuncOpLive>>
) -> ExecResult<()> {
    bind_args_to_context(mmu.clone(), func, args, context.clone(), op_queue.clone())?;
    handle_constants(mmu, func, context, op_queue)?;
    Ok(())
}

/// Takes the arguments to a function as a list of value references, gets their corresponding func value nodes, and binds them to the context.
fn bind_args_to_context<'a>(mmu: Arc<MMU>,
                            func: &FuncLive,
                            args: &[ValueReference],
                            context: Arc<RwLock<HashMap<Symbol, ValueReference>>>,
                            op_queue: Arc<SegQueue<FuncOpLive>>
) -> ExecResult<()> {
    for (i, arg_value) in args.iter().enumerate() {
        let input_ptr = func.input_vals.get(i)
            .ok_or("Function input value missing")?;
        let input = get_func_val(mmu.clone(), input_ptr)
            .map_err(|msg| format!("Function cannot get input value: {}", msg))?;

        context.write().unwrap().insert(input.guid.clone(), arg_value.clone());
        let new_ops = handle_func_val_dependents(mmu.clone(), &input, context.clone())?;
        for new_op in new_ops {
            op_queue.push(new_op);
        }
    }
    Ok(())
}

/// Handles the constant values present in a function's scope by retrieving their values and binding them to the context.
fn handle_constants(mmu: Arc<MMU>,
                    func: &FuncLive,
                    context: Arc<RwLock<HashMap<Symbol, ValueReference>>>,
                    op_queue: Arc<SegQueue<FuncOpLive>>
) -> ExecResult<()> {
    for constant_ptr in &func.constant_vals {
        let constant_val = get_func_val(mmu.clone(), constant_ptr)
            .map_err(|msg| format!("Function cannot get constant value: {}", msg))?;

        let constant_ref = match constant_val.constant.as_ref() {
            Some(ptr) => value_ref_from_ptr(mmu.clone(), ptr.clone())?,
            None => return Err("Function expected constant but none found".to_string())
        };

        context.write().unwrap().insert(constant_val.guid.clone(), constant_ref);
        let new_ops = handle_func_val_dependents(mmu.clone(), &constant_val, context.clone())?;
        for new_op in new_ops {
            op_queue.push(new_op);
        }
    }
    Ok(())
}


/// Executes an operation within the scope of a function using the provided context.
/// Retrieves the arg values from the context, executes the operation, and returns the result values.
fn try_execute_fn_op(mmu: Arc<MMU>, op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<Vec<ValueReference>> {
    validate_op_inputs(mmu.clone(), op, context.clone())?;
    let result_val_refs = handle_call_function_op(mmu.clone(), op, context.clone())
        .map_err(|msg| format!("Execution of operation {} failed: {}", op.opcode, msg))?;

    if result_val_refs.len() != op.output_vals.len() {
        return Err(format!("Function operation expected {} result values, but got {}", op.output_vals.len(), result_val_refs.len()));
    }

    // get the write lock on the context
    let mut context = context.write().unwrap();

    // loop through the outputs from the executed operation
    for (output_ptr, result_val_ref) in op.output_vals.iter().zip(&result_val_refs) {
        // get the output
        let output_val = get_func_val(mmu.clone(), output_ptr)
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
fn validate_op_inputs(mmu: Arc<MMU>, op: &FuncOpLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<()> {
    // get the read lock on the context
    let context = context.read().unwrap();

    op.input_vals.iter().enumerate().try_for_each(|(arg_index, input_ptr)| {
        let input_val = get_func_val(mmu.clone(), input_ptr)
            .map_err(|msg| format!("Function cannot get input value (index {}): {}", arg_index, msg))?;

        if !context.contains_key(&input_val.guid) {
            Err(format!("Operation {} cannot be executed because input value index {} is not known", op.opcode, arg_index))
        } else {
            Ok(())
        }
    })
}

/// Manages the call of function operations and the queuing of its dependent operations for execution.
fn manage_op_queue(
    mmu: Arc<MMU>,
    op_queue: Arc<SegQueue<FuncOpLive>>,
    context: Arc<RwLock<HashMap<Symbol, ValueReference>>>
) -> ExecResult<()> {
    let thread_pool = rayon::ThreadPoolBuilder::new().build().unwrap();

    while let Some(op) = op_queue.pop() {
        let (tx, rx) = channel();
        // Spawning a new thread to execute the operation
        thread_pool.scope(|s| {
            let context = context.clone();
            let op = op.clone();
            let mmu = mmu.clone();

            s.spawn(move |_| {
                let result = try_execute_fn_op(mmu.clone(), &op, context.clone());
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
                let output_val = get_func_val(mmu.clone(), output_ptr)
                    .map_err(|msg| format!("Function cannot get output value for operation {}: {}", op.opcode, msg))?;

                let new_ops = handle_func_val_dependents(mmu.clone(), &output_val, context.clone())
                    .map_err(|msg| format!("Function cannot handle dependents for operation {}: {}", op.opcode, msg))?;

                for new_op in new_ops {
                    op_queue.push(new_op);
                }
            }
        }
    }

    Ok(())
}

fn get_func_call_outputs(mmu: Arc<MMU>, func: &FuncLive, context: Arc<RwLock<HashMap<Symbol, ValueReference>>>) -> ExecResult<Vec<ValueReference>> {
    // obtain the read lock on the context
    let context = context.read().unwrap();

    func.output_vals.iter()
        .map(|output_ptr| {
            let output_val = get_func_val(mmu.clone(), output_ptr)
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

fn validate_func_inputs(mmu: Arc<MMU>, func: &FuncLive, args: &[ValueReference]) -> ExecResult<()> {
    // Check if the function has the right number of args
    if func.input_vals.len() != args.len() {
        return Err(format!("Function expected {} arguments, but got {}", func.input_vals.len(), args.len()));
    }
    Ok(())
}

/// Handles the call of a function.
pub fn handle_call_function(mmu: Arc<MMU>, func: &FuncLive, args: &[ValueReference]) -> ExecResult<Vec<ValueReference>> {
    match validate_func_inputs(mmu.clone(), func, args) {
        Ok(_) => (),
        Err(msg) => return Err(format!("Function call failed: {}", msg))
    }

    let context: Arc<RwLock<HashMap<Symbol, ValueReference>>> = Arc::new(RwLock::new(HashMap::new()));
    let op_queue: Arc<SegQueue<FuncOpLive>> = Arc::new(SegQueue::new());

    // Initialize the function call
    initialize_func_call(mmu.clone(), func, args, context.clone(), op_queue.clone())?;

    // Execute the function's operations
    manage_op_queue(mmu.clone(), op_queue, context.clone())?;

    get_func_call_outputs(mmu.clone(), func, context)
}

pub fn execute_call(mmu: Arc<MMU>, func: &ValueReference, args: Vec<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
    // get the function
    let func = match mmu.get_ref_value(func) {
        Ok(val) => val,
        Err(msg) => return Err(format!("Failed to get function: {}", msg))
    };
    let func = func.as_live().as_func().ok_or_else(|| "Cannot call a non-function value".to_string())??;

    // get the args and ensure that there are the correct number of them
    let mut args_cloned: Vec<ValueReference> = Vec::new();
    for arg in args {
        args_cloned.push(clone_reference(mmu.clone(), arg)?);
    }

    let result = handle_call_function(mmu.clone(), &func, &args_cloned);

    result
}


// #[cfg(test)]
// mod tests {
//     use std::collections::HashMap;
//     use crate::api::collections::collection::Collection;
//     use crate::api::GraphiteApi;
//     use crate::api::interface::VmInterface;
//     use crate::core::data::live::IntLive;
//     use crate::core::vm::VM;
//     use crate::core::data::live::live_data::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
//     use crate::core::ExecResult;
//     use crate::core::vm::value_ref::ValueReference;
//
//     // tests calling a function and receiving results asynchronously
//     #[test]
//     fn test_call_async<'a>() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };
//
//             let json_collection = r#"{
//                 "constants": {},
//                 "functions": {
//                     "main": {
//                         "graph": {
//                             "values": [
//                                 "initial",
//                                 "a",
//                                 "b",
//                                 "c",
//                                 "d",
//                                 ["factor", 2]
//                             ],
//                             "ops": [
//                                 ["Add", ["c", "factor"], "a"],
//                                 ["Add", ["d", "factor"], "b"],
//                                 ["Add", ["b", "factor"], "c"],
//                                 ["Mul", ["initial", "factor"], "d"]
//                             ],
//                             "input_vals": ["initial"],
//                             "output_vals": ["a", "b", "c", "d"]
//                         }
//                     }
//                 },
//                 "collections": {},
//                 "imports": {}
//             }"#;
//
//             let collection: Collection = match serde_json::from_str(json_collection) {
//                 Ok(collection) => collection,
//                 Err(e) => {
//                     println!("{}", e);
//                     panic!();
//                 }
//             };
//
//             api.store_collection(collection, "my_collection".to_string()).unwrap();
//             let main_func_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();
//             let main_func = vm.get_ref_value(&main_func_ref).unwrap().as_live().as_func().unwrap().ok().unwrap();
//             drop(main_func_ref);
//             let initial: IntLive = 5.into();
//             api.store_int(initial, "initial".to_string()).unwrap();
//             let initial_ref = api.get_path(vec!["initial".into()]).unwrap();
//
//             // results should be calculated in the order of d, b, c, a
//             let expected_order =vec![(3, 10), (1, 12), (2, 14), (0, 16)];
//
//             let mut results: Vec<IntLive> = Vec::new();
//
//             let callback = |i: usize, result: ValueReference| -> ExecResult<()> {
//                 let result = vm.get_ref_value(&result).unwrap().as_live().as_int().unwrap().ok().unwrap();
//                 results.push(result);
//                 Ok(())
//             };
//
//             vm.handle_call_function_async(&main_func, &[initial_ref], callback).unwrap();
//
//             assert_eq!(results.len(), 4);
//             for (i, result) in results.iter().enumerate() {
//                 let expected = expected_order[i].1;
//                 assert_eq!(*result, expected);
//             }
//         }
//
//         assert_eq!(vm.object_count(), 0);
//     }
// }