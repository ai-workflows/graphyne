use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;
use crate::runtime::data::live::{BoolLive, FuncOpLive, FuncValLive, ListLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::functions::op::FuncOpId;
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::stored::StoredData;
use crate::runtime::mmu::mmu::{execute_store, MMU, value_ref_from_ptr};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::operator::execute_op;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::manager::{manage_await_call, manage_start_call, StreamResult};
use crate::runtime::vm::shared::{CallContextId, get_func_vals_from_ptrs, NewValMessage, SharedCallState, ValPendingMessage, ExecutorMessage, CallResult};

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
        OpCode::Reduce => handle_reduce_op(shared_state.clone(), get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?),
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
    args.iter().enumerate().try_for_each(|(arg_index, arg_fn_val)| {
        if !shared_state.contains_val(call_context_id, arg_fn_val) {
            Err(format!("Arg at index {} is not known.", arg_index))
        } else {
            Ok(())
        }
    })
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
    args.iter()
        .map(move |arg_fn_val| {
            match shared_state.get_val(call_context_id, arg_fn_val) {
                Some(val) => Ok(val),
                None => Err("Arg value not found in state and not caught by validation".to_string())
            }
        })
        .collect()
}


/// TODO: allow passing of other values into the reduce function other than the list items
/// Pass in an arg "reduce_func_args" which can either be a ref to a func val or a special enum
/// value that indicates this is the arg for the list items
fn handle_reduce_op(shared_state: Arc<SharedCallState>, args: Vec<ValueReference>) -> ExecResult<Vec<ValueReference>> {
    if args.len() != 3 {
        return Err("Reduce operation requires 3 arguments".to_string());
    }

    let func = args[0].clone();
    let list = match shared_state.mmu.get_ref_value(&args[1]) {
        Ok(StoredData::ListStored(list)) => list,
        _ => return Err("Reduce operation requires a list as the second arg".to_string())
    };
    let mut current = args[2].clone();

    for item_ptr in list.iter() {
        let item_ref = value_ref_from_ptr(shared_state.mmu.clone(), item_ptr.clone())?;

        let args: Vec<ValueReference> = vec![current.clone(), item_ref];

        let result = manage_await_call(
            shared_state.mmu.clone(),
            shared_state.executor_thread_pool.clone(),
            shared_state.orchestrator_thread_pool.clone(),
            func.clone(),
            args,
            shared_state.verbose,
        );

        match result {
            Ok(result) => {
                current = result.get(0).ok_or("Empty result")?.clone();
            },
            Err(msg) => return Err(format!("Error executing reduce function: {}", msg))
        }
    }

    Ok(vec![current])
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
    let list = match shared_state.mmu.get_ref_value(&args[1]) {
        Ok(StoredData::ListStored(list)) => list,
        _ => return Err("Map operation requires a list as the second arg".to_string())
    };

    let output_fn_val: FuncValLive = match shared_state.mmu.get_ptr_value(&op.output_vals[0]) {
        Ok(StoredData::FuncValStored(func_val)) => func_val,
        _ => return Err("Output func val not found".to_string())
    };

    dispatch_map(shared_state.clone(), call_context_id.clone(), func, list, output_fn_val.clone(), op.guid.clone());

    Ok(vec![ExecutorMessage::Pending(ValPendingMessage {
        call_context_id: call_context_id.clone(),
        func_val: output_fn_val.clone()
    })])
}

fn dispatch_map(
    shared_state: Arc<SharedCallState>,
    call_context_id: CallContextId,
    func: ValueReference,
    list: ListLive,
    output_fn_val: FuncValLive,
    op_id: FuncOpId
) {
    thread::spawn(move || {
        // dispatch calls for each item in the list
        let results = match dispatch_calls(
            shared_state.clone(),
            &list,
            func.clone())
        {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        // store the results in a list
        let result_list = match store_list(shared_state.mmu.clone(), results) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        // remove the operation from the pending list
        shared_state.complete_pending_op(&call_context_id, &op_id);

        // send the result list as a new value
        shared_state.send_new_val(call_context_id.clone(), output_fn_val.clone(), result_list);
    });
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
    let list = match shared_state.mmu.get_ref_value(&args[1]) {
        Ok(StoredData::ListStored(list)) => list,
        _ => return Err("Filter operation requires a list as the second arg".to_string())
    };

    let output_fn_val: FuncValLive = match shared_state.mmu.get_ptr_value(&op.output_vals[0]) {
        Ok(StoredData::FuncValStored(func_val)) => func_val,
        _ => return Err("Output func val not found".to_string())
    };

    dispatch_filter(shared_state.clone(), call_context_id.clone(), func, list, output_fn_val.clone(), op.guid.clone());

    Ok(vec![ExecutorMessage::Pending(ValPendingMessage {
        call_context_id: call_context_id.clone(),
        func_val: output_fn_val.clone()
    })])
}

fn dispatch_filter(
    shared_state: Arc<SharedCallState>,
    call_context_id: CallContextId,
    func: ValueReference,
    list: ListLive,
    output_fn_val: FuncValLive,
    op_id: FuncOpId
) {
    thread::spawn(move || {
        // dispatch calls for each item in the list
        let results = dispatch_calls(shared_state.clone(), &list, func.clone())
            .and_then(|results| {
                // Convert and validate values together
                results
                    .iter()
                    .map(|val| {
                        shared_state
                            .mmu
                            .get_ref_value(val)
                            .and_then(|stored_data| match stored_data {
                                StoredData::BoolStored(b) => Ok(b),
                                _ => Err("Expected filter function to return a boolean value".to_string()),
                            })
                    })
                    .collect::<Result<Vec<BoolLive>, String>>()
            });

        // halt and stop if there was an error
        if let Err(err_msg) = results {
            shared_state.halt_execution(&call_context_id, CallResult::Error(err_msg));
            return;
        }
        // only keep results if the results is true
        let filtered_list: Vec<ValueReference> = list.iter()
            .zip(results.as_ref().unwrap()) // Zip iterators for simultaneous access
            .filter_map(|(list_item, is_true)| {
                if *is_true {
                    Some(value_ref_from_ptr(shared_state.mmu.clone(), list_item.clone()).unwrap())
                } else {
                    None // Filter out the item
                }
            })
            .collect();

        // store the results in a list
        let result_list = match store_list(shared_state.mmu.clone(), filtered_list) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        // remove the operation from the pending list
        shared_state.complete_pending_op(&call_context_id, &op_id);

        // send the result list as a new value
        shared_state.send_new_val(call_context_id.clone(), output_fn_val.clone(), result_list);
    });
}

fn dispatch_calls(
    shared_state: Arc<SharedCallState>,
    list: &ListLive,
    func: ValueReference,
) -> ExecResult<Vec<ValueReference>> {
    // set up the list of results receivers
    let mut output_receivers: Vec<Receiver<StreamResult>> = Vec::with_capacity(list.len());

    // dispatch calls for each item in the list
    for item_ptr in list.iter(){
        let ss = shared_state.clone();

        // send the item from the list as an arg
        let args: Vec<ValueReference> = vec![value_ref_from_ptr(ss.mmu.clone(), item_ptr.clone()).unwrap()];

        // set up output channel
        let (output_sender, output_receiver) = mpsc::channel::<StreamResult>();

        // start the call and add the receiver to the list
        let num_expected_outputs = manage_start_call(
            shared_state.mmu.clone(),
            shared_state.executor_thread_pool.clone(),
            shared_state.orchestrator_thread_pool.clone(),
            func.clone(),
            args,
            Arc::new(Mutex::new(output_sender)),
            shared_state.verbose
        );

        match num_expected_outputs {
            Ok(c) => if c != 1 {
                return Err("Expected 1 output from each call".to_string());
            },
            Err(msg) => return Err(msg)
        }

        output_receivers.push(output_receiver);
    }

    // set up the output list
    let mut outputs: Vec<(usize, ValueReference)> = Vec::with_capacity(list.len());

    // wait until all calls are finished
    for (i, rec) in output_receivers.iter().enumerate() {
        match rec.recv() {
            Ok(StreamResult::Output(_, val_ref)) => outputs.push((i, val_ref)),
            Ok(StreamResult::Error(msg)) => return Err(msg),
            Err(_) => return Err("Error receiving result".to_string())
        }
    }


    // sort the results and return them
    let sorted = sort_results(outputs);
    Ok(sorted)
}

fn collect_results(
    list: ListLive,
    rx: Receiver<ExecResult<(usize, ValueReference)>>
) -> ExecResult<Vec<(usize, ValueReference)>> {
    let unsorted_results: Mutex<Vec<(usize, ValueReference)>> = Mutex::new(Vec::with_capacity(list.len()));

    for result in rx.iter().take(list.len()) {
        match result {
            Ok((i, val)) => {
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