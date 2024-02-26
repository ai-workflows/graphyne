use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;
use crate::runtime::data::live::{BoolLive, FuncOpLive, FuncValLive, ListLive};
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::stored::StoredData;
use crate::runtime::mmu::mmu::{execute_store, MMU, value_ref_from_ptr};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::operator::execute_op;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::manager::{manage_await_call, manage_start_call};
use crate::runtime::vm::shared::{CallContextId, get_func_vals_from_ptrs, NewValMessage, SharedCallState, ValPendingMessage, ExecutorMessage, send_new_val};

/// A worker responsible for executing

/// Executes an operation within the scope of a function call context.
/// Retrieves the arg values from the state, executes the operation, and returns the result values.
pub fn try_execute_fn_op(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    call_context_id: &CallContextId,
) -> ExecResult<Vec<ExecutorMessage>> {
    shared_state.log_async(call_context_id, &format!("Executing operation: {}", op.opcode));

    // get the func vals for the arguments
    let arg_fn_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.input_vals) {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error getting input func vals for operation: {}", msg))
    };

    validate_op_inputs(&shared_state, &arg_fn_vals, call_context_id)?;

    let res = match op.opcode {
        OpCode::Reduce => return handle_reduce_op(
            shared_state.clone(),
            get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?,
            call_context_id,
            op
        ),
        OpCode::Map => return handle_map_op(
            shared_state.clone(),
            get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?,
            call_context_id,
            op
        ),
        OpCode::Filter => return handle_filter_op(
            shared_state.clone(),
            get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?,
            call_context_id,
            op
        ),
        _ => handle_call_function_op(shared_state.clone(), &op.opcode, &arg_fn_vals, call_context_id)
    };

    let result_val_refs: Vec<ValueReference> = match res {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error executing operation: {}", msg))
    };

    if result_val_refs.len() != op.output_vals.len() {
        return Err(format!("Operation expected {} result values, but got {}", op.output_vals.len(), result_val_refs.len()));
    }

    // get the output func vals
    let output_func_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.output_vals) {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error getting output func vals for operation: {}", msg))
    };

    // match the output func vals with the result val refs
    let result: Vec<ExecutorMessage> = result_val_refs.iter()
        .zip(output_func_vals)
        .map(|(val_ref, func_val)|
            ExecutorMessage::NewVal(
                NewValMessage {
                    call_context_id: call_context_id.clone(),
                    value: val_ref.clone(),
                    func_val: func_val.clone()}))
        .collect();

    shared_state.log_async(call_context_id, &format!(
        "Operation ({}) executed successfully with result: {:?}",
        op.opcode,
        match result[0] {
            ExecutorMessage::NewVal(ref msg) => msg.func_val.symbol.clone().unwrap(),
            _ => "Unknown".to_string()
    }));

    Ok(result)
}

/// Validates the inputs to function's operation by checking that all args are present in the context.
fn validate_op_inputs(shared_state: &Arc<SharedCallState>, args: &Vec<FuncValLive>, call_context_id: &CallContextId) -> ExecResult<()> {
    if !shared_state.contains_all_vals(call_context_id, args) {
        return Err("Not all input values are known.".to_string());
    }

    // args.iter().enumerate().try_for_each(|(arg_index, arg_fn_val)| {
    //     if !shared_state.contains_val(call_context_id, arg_fn_val) {
    //         Err(format!("Arg at index {} is not known.", arg_index))
    //     } else {
    //         Ok(())
    //     }
    // })

    Ok(())
}

/// Handles the call of an operation that is part of a function.
/// Gets the arguments from the context, executes the operation, and returns the result values.
pub fn handle_call_function_op(shared_state: Arc<SharedCallState>, op_code: &OpCode, args: &Vec<FuncValLive>, call_context_id: &CallContextId, ) -> ExecResult<Vec<ValueReference>> {
    let arg_values: Vec<ValueReference> = get_func_op_args(shared_state.clone(), args, call_context_id)?;
    let arg_values: Vec<&ValueReference> = arg_values.iter().collect();
    let op: Operation = op_code.to_operation(&arg_values);
    
    execute_op(shared_state.mmu.clone(), op)
}

/// Gets the arguments to a function's operation from the state manager
fn get_func_op_args(shared_state: Arc<SharedCallState>, args: &Vec<FuncValLive>, call_context_id: &CallContextId) -> ExecResult<Vec<ValueReference>> {
    let res: Vec<Option<ValueReference>> = shared_state.get_vals(call_context_id, args);
    // if any of the args are not found, return an error
    if res.iter().any(|val| val.is_none()) {
        return Err("Not all input values are known.".to_string());
    }

    Ok(res.into_iter().map(|val| val.unwrap()).collect())
}


/// TODO: allow passing of other values into the reduce function other than the list items
/// Pass in an arg "reduce_func_args" which can either be a ref to a func val or a special enum
/// value that indicates this is the arg for the list items
fn handle_reduce_op(
    shared_state: Arc<SharedCallState>,
    args: Vec<ValueReference>,
    call_context_id: &CallContextId,
    op: &FuncOpLive,
) -> ExecResult<Vec<ExecutorMessage>> {
    if args.len() != 3 {
        return Err("Reduce operation requires 3 arguments".to_string());
    }

    let func = args[0].clone();
    let list_ref = shared_state.mmu.get_ref_value(&args[1])?;
    let list: ListLive = match list_ref.as_ref() {
        StoredData::ListStored(list) => list.clone(),
        _ => return Err("Reduce operation requires a list as the second arg".to_string())
    };

    let output_fn_val_ref = shared_state.mmu.get_ptr_value(&op.output_vals[0])?;
    let output_fn_val: FuncValLive = match output_fn_val_ref.as_ref() {
        StoredData::FuncValStored(func_val) => func_val.clone(),
        _ => return Err("Output func val not found".to_string())
    };

    // spawn a new thread so that orchestrating the reduce operation doesn't block a worker
    let cc_id = call_context_id.clone();
    let op = op.clone();
    let output_fn_val2 = output_fn_val.clone();
    thread::spawn(move || {
        let result = match dispatch_reduce(shared_state.clone(), func, &list, args[2].clone()) {
            Ok(val) => val,
            Err(msg) => {
                shared_state.throw_error(&cc_id.clone(), &msg);
                return;
            }
        };

        // remove the operation from the pending list
        shared_state.complete_pending_op(&cc_id, &op.guid);

        // send the result list as a new value
        send_new_val(shared_state.clone(), cc_id.clone(), &output_fn_val2, result);
    });

    // return pending message
    Ok(vec![ExecutorMessage::Pending(ValPendingMessage {
        call_context_id: call_context_id.clone(),
        func_val: output_fn_val
    })])
}

fn dispatch_reduce(
    shared_state: Arc<SharedCallState>,
    func: ValueReference,
    list: &ListLive,
    initial: ValueReference,
) -> ExecResult<ValueReference> {
    let mut current = initial;

    for item_ptr in list.iter() {
        let item_ref = value_ref_from_ptr(shared_state.mmu.clone(), item_ptr.clone())?;

        let args: Vec<ValueReference> = vec![current.clone(), item_ref];

        let result = manage_await_call(
            shared_state.mmu.clone(),
            shared_state.worker_pool.clone(),
            func.clone(),
            args,
            shared_state.verbose,
        );

        match result {
            Ok(result) => {
                if result.len() != 1 {
                    return Err("Error executing reduce function: Expected 1 result value".to_string());
                }

                // get the only result value from the hash map
                let res_key: &Symbol = match result.keys().next() {
                    Some(val) => val,
                    None => {
                        return Err("Error executing reduce function: No result value".to_string());
                    }
                };

                current = match result.get(res_key) {
                    Some(val) => val.clone(),
                    None => {
                        return Err("Error executing reduce function: No result value".to_string());
                    }
                }

                // current = match result.get(0) {
                //     Some(val) => val.clone(),
                //     None => {
                //         return Err("Error executing reduce function: No result value".to_string());
                //     }
                // }
            },
            Err(msg) => {
                return Err(format!("Error executing reduce function: {}", msg));
            }
        }
    }

    Ok(current)
}

fn handle_map_op(
    shared_state: Arc<SharedCallState>,
    args: Vec<ValueReference>,
    call_context_id: &CallContextId,
    op: &FuncOpLive,
) -> ExecResult<Vec<ExecutorMessage>> {
    if args.len() != 2 {
        return Err("Map operation requires 2 arguments".to_string());
    }

    let func = args[0].clone();

    let list_ref = shared_state.mmu.get_ref_value(&args[1])?;
    let list: ListLive = match list_ref.as_ref() {
        StoredData::ListStored(list) => list.clone(),
        _ => return Err("Map operation requires a list as the second arg".to_string())
    };

    let output_fn_val_ref = shared_state.mmu.get_ptr_value(&op.output_vals[0])?;
    let output_fn_val: FuncValLive = match output_fn_val_ref.as_ref() {
        StoredData::FuncValStored(func_val) => func_val.clone(),
        _ => return Err("Output func val not found".to_string())
    };

    let cc_id = call_context_id.clone();
    let op = op.clone();
    let lis = list.clone();
    let output_fn_val2 = output_fn_val.clone();
    thread::spawn(move || {
        let result = match dispatch_map(shared_state.clone(), func, &lis) {
            Ok(val) => val,
            Err(msg) => {
                shared_state.throw_error(&cc_id.clone(), &msg);
                return;
            }
        };

        // remove the operation from the pending list
        shared_state.complete_pending_op(&cc_id, &op.guid);

        // send the result list as a new value
        send_new_val(shared_state.clone(), cc_id.clone(), &output_fn_val2, result);
    });

    Ok(vec![ExecutorMessage::Pending(ValPendingMessage {
        call_context_id: call_context_id.clone(),
        func_val: output_fn_val
    })])
}

fn dispatch_map(
    shared_state: Arc<SharedCallState>,
    func: ValueReference,
    list: &ListLive,
) -> ExecResult<ValueReference> {
    // dispatch calls for each item in the list
    let results = dispatch_calls(
        shared_state.clone(),
        &list,
        func.clone())?;

    // store the results in a list
    let result_list = store_list(shared_state.mmu.clone(), results)?;

    Ok(result_list)
}

fn handle_filter_op(
    shared_state: Arc<SharedCallState>,
    args: Vec<ValueReference>,
    call_context_id: &CallContextId,
    op: &FuncOpLive,
) -> ExecResult<Vec<ExecutorMessage>> {
    if args.len() != 2 {
        return Err("Filter operation requires 2 arguments".to_string());
    }

    let func = args[0].clone();

    let list_ref = shared_state.mmu.get_ref_value(&args[1])?;
    let list: ListLive = match list_ref.as_ref() {
        StoredData::ListStored(list) => list.clone(),
        _ => return Err("Filter operation requires a list as the second arg".to_string())
    };

    let output_fn_val_ref = shared_state.mmu.get_ptr_value(&op.output_vals[0])?;
    let output_fn_val: FuncValLive = match output_fn_val_ref.as_ref() {
        StoredData::FuncValStored(func_val) => func_val.clone(),
        _ => return Err("Output func val not found".to_string())
    };

    // spawn a new thread so that orchestrating the filter operation doesn't block a worker
    let cc_id = call_context_id.clone();
    let op = op.clone();
    let output_fn_val2 = output_fn_val.clone();
    thread::spawn(move || {
        let result = match dispatch_filter(shared_state.clone(), func, &list) {
            Ok(val) => val,
            Err(msg) => {
                shared_state.throw_error(&cc_id.clone(), &msg);
                return;
            }
        };

        // remove the operation from the pending list
        shared_state.complete_pending_op(&cc_id, &op.guid);

        // send the result list as a new value
        send_new_val(shared_state.clone(), cc_id.clone(), &output_fn_val, result);
    });

    Ok(vec![ExecutorMessage::Pending(ValPendingMessage {
        call_context_id: call_context_id.clone(),
        func_val: output_fn_val2
    })])
}

fn dispatch_filter(
    shared_state: Arc<SharedCallState>,
    func: ValueReference,
    list: &ListLive,
) -> ExecResult<ValueReference> {
    // dispatch calls for each item in the list
    let results: Vec<BoolLive> = dispatch_calls(shared_state.clone(), &list, func.clone())
        .and_then(|results| {
            // Convert and validate values together
            results
                .iter()
                .map(|val| {
                    shared_state
                        .mmu
                        .get_ref_value(val)
                        .and_then(|stored_data| match stored_data.as_ref() {
                            StoredData::BoolStored(b) => Ok(b.clone()),
                            _ => Err("Expected filter function to return a boolean value".to_string()),
                        })
                })
                .collect::<Result<Vec<BoolLive>, String>>()
        })?;

    // only keep results if the results is true
    let filtered_list: Vec<ValueReference> = list.iter()
        .zip(results) // Zip iterators for simultaneous access
        .filter_map(|(list_item, is_true)| {
            if is_true {
                Some(value_ref_from_ptr(shared_state.mmu.clone(), list_item.clone()).unwrap())
            } else {
                None // Filter out the item
            }
        })
        .collect();

    // store the results in a list
    store_list(shared_state.mmu.clone(), filtered_list)
}

fn dispatch_calls(
    shared_state: Arc<SharedCallState>,
    list: &ListLive,
    func: ValueReference,
) -> ExecResult<Vec<ValueReference>> {
    let (tx, rx) = std::sync::mpsc::channel();

    // dispatch calls for each item in the list
    for (i, item_ptr) in list.iter().enumerate() {
        let ss = shared_state.clone();
        let f = func.clone();
        let item_ptr = item_ptr.clone();
        let tx = tx.clone();

        thread::spawn(move || {
            // send the item from the list as an arg
            let args: Vec<ValueReference> = vec![value_ref_from_ptr(ss.mmu.clone(), item_ptr).unwrap()];

            // start the call and add the receiver to the list
            let result: ExecResult<HashMap<Symbol, ValueReference>> = manage_await_call(
                ss.mmu.clone(),
                ss.worker_pool.clone(),
                f,
                args,
                ss.verbose
            );

            let result = match result {
                Ok(res) => res,
                Err(msg) => {
                    tx.send((i, Err(msg))).unwrap();
                    return;
                }
            };

            let msg = match result.len() {
                0 => (i, Err("No result value returned".to_string())),
                1 => (i, Ok(result.values().next().unwrap().clone())),
                _ => (i, Err("Too many result values returned".to_string()))
            };

            match tx.send(msg) {
                Ok(_) => (),
                Err(e) => {
                    ss.throw_error(&CallContextId::new(), &format!("Error sending result: {}", e));
                }
            }
        });
    }

    // collect the results
    let results = collect_results(list.clone(), rx)?;

    // sort the results by the original index
    let results: Vec<ValueReference> = sort_results(results);

    Ok(results)
}

fn collect_results(
    list: ListLive,
    rx: Receiver<(usize, ExecResult<ValueReference>)>
) -> ExecResult<Vec<(usize, ValueReference)>> {
    let unsorted_results: Mutex<Vec<(usize, ValueReference)>> = Mutex::new(Vec::with_capacity(list.len()));

    for result in rx.iter().take(list.len()) {
        let i = result.0;
        let result = result.1;
        match result {
            Ok(val) => {
                let mut results = unsorted_results.lock().unwrap();
                results.push((i, val));
            },
            Err(msg) => return Err(msg)
        }
    }

    Ok(unsorted_results.into_inner().unwrap())
}

fn sort_results(results: Vec<(usize, ValueReference)>) -> Vec<ValueReference> {
    let mut results = results;
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results.into_iter().map(|(_, val)| val).collect()
}

fn store_list(mmu: Arc<MMU>, values: Vec<ValueReference>) -> ExecResult<ValueReference> {
    let values: Vec<&ValueReference> = values.iter().collect();
    let store_op: StoreOp = StoreOp::StoreList(values);
    let res = match execute_store(mmu.clone(), store_op) {
        Ok(res) => res,
        Err(msg) => return Err(msg)
    };

    Ok(res.get(0).unwrap().clone())
}

// Executes a function synchronously
// fn execute_call_sync()