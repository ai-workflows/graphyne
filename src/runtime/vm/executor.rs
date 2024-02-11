use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use crate::runtime::data::live::{FuncOpLive, FuncValLive, ListLive};
use crate::runtime::{ExecResult};
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::stored::StoredData;
use crate::runtime::mmu::mmu::{execute_store, MMU, value_ref_from_ptr};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::operator::execute_op;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::manager::{await_call, start_call};
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

        let result = await_call(
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

    dispatch_map(shared_state.clone(), call_context_id.clone(), func, list, output_fn_val.clone());

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
    output_fn_val: FuncValLive
) {
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let shared_tx = Arc::new(tx);

        dispatch_calls(shared_state.clone(), list.clone(), func.clone(), shared_tx.clone());

        // collect the results
        let unsorted = match collect_results(list, rx) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        // sort and return the results
        let sorted = sort_results(unsorted);
        let result = match store_list(shared_state.mmu.clone(), sorted) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        shared_state.send_new_val(call_context_id.clone(), output_fn_val.clone(), result);
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

    dispatch_filter(shared_state.clone(), call_context_id.clone(), func, list, output_fn_val.clone());

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
    output_fn_val: FuncValLive
) {
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let shared_tx = Arc::new(tx);

        // dispatch calls for each item in the list
        dispatch_calls(shared_state.clone(), list.clone(), func.clone(), shared_tx.clone());

        // collect the results
        let unsorted = match collect_results(list, rx) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        // only keep results if the results is true
        let unsorted: Vec<(usize, ValueReference)> = unsorted.into_iter().filter(|(_, val)| {
            match shared_state.mmu.get_ref_value(val) {
                Ok(StoredData::BoolStored(b)) => b,
                _ => {
                    shared_state.halt_execution(
                        &call_context_id,
                        CallResult::Error("Expected filter function to return a boolean value".to_string()));
                    return false;
                }
            }
        }).collect();

        // sort the results and return them
        let sorted = sort_results(unsorted);
        let result = match store_list(shared_state.mmu.clone(), sorted) {
            Ok(res) => res,
            Err(msg) => {
                shared_state.halt_execution(&call_context_id, CallResult::Error(msg));
                return;
            }
        };

        shared_state.send_new_val(call_context_id.clone(), output_fn_val.clone(), result);
    });
}

fn dispatch_calls(
    shared_state: Arc<SharedCallState>,
    list: ListLive,
    func: ValueReference,
    results_tx: Arc<mpsc::Sender<ExecResult<(usize, ValueReference)>>>,
) {
    // let mmu = shared_state.mmu.clone();
    for (i, item_ptr) in list.iter().enumerate() {
        let ss = shared_state.clone();

        let item_ref = value_ref_from_ptr(ss.mmu.clone(), item_ptr.clone()).unwrap();

        let shared_tx = results_tx.clone();
        let shared_tx2 = shared_tx.clone();

        let output_callback = Arc::new(move |message: &NewValMessage| {
            let tx2 = shared_tx.clone();
            let ss = ss.clone();

            match tx2.send(Ok((i, message.value.clone()))) {
                Ok(_) => (),
                Err(e) => ss.halt_execution(
                    &CallContextId::new(),
                    CallResult::Error(format!{"Error sending output to results channel: {}", e}))
            }
        });

        let ss = shared_state.clone();

        let result_callback = Arc::new(move |result: ExecResult<()>| {
            let tx2 = shared_tx2.clone();

            if let Err(e) = result {
                let e2 = e.clone();
                match tx2.send(Err(e)) {
                    Ok(_) => (),
                    Err(se) => ss.halt_execution(
                        &CallContextId::new(),
                        CallResult::Error(format!{"Error sending error ({}) to results channel: {}", e2, se}))
                }
            }
        });

        let args: Vec<ValueReference> = vec![item_ref];

        start_call(
            shared_state.mmu.clone(),
            shared_state.executor_thread_pool.clone(),
            shared_state.orchestrator_thread_pool.clone(),
            func.clone(),
            args,
            output_callback,
            result_callback,
            shared_state.verbose,
        );
    }
}

fn collect_results(
    list: ListLive,
    rx: mpsc::Receiver<ExecResult<(usize, ValueReference)>>
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