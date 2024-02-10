use std::sync::{Arc, mpsc, Mutex};
use crate::core::data::live::{FuncOpLive, FuncValLive};
use crate::core::{ExecResult};
use crate::core::data::functions::OpCode;
use crate::core::data::stored::StoredData;
use crate::core::vm::mmu::mmu::{execute_store, value_ref_from_ptr};
use crate::core::vm::mmu::store_op::StoreOp;
use crate::core::vm::operator::operator::execute_op;
use crate::core::vm::operator::ops::Operation;
use crate::core::vm::v2::manager::{await_call, start_call};
use crate::core::vm::v2::shared::{CallContextId, get_func_vals_from_ptrs, log_async, NewValMessage, SharedCallState};
use crate::core::vm::value_ref::ValueReference;

/// A worker responsible for executing

/// Executes an operation within the scope of a function call context.
/// Retrieves the arg values from the state, executes the operation, and returns the result values.
pub fn try_execute_fn_op(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    call_context_id: &CallContextId,
) -> ExecResult<Vec<(ValueReference, FuncValLive)>> {
    log_async(call_context_id, &format!("Executing operation: {}", op.opcode));

    // get the func vals for the arguments
    let arg_fn_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.input_vals) {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error getting input func vals for operation: {}", msg))
    };

    validate_op_inputs(&shared_state, &arg_fn_vals, call_context_id)?;

    let res = match op.opcode {
        OpCode::Reduce => handle_reduce_op(shared_state.clone(), get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?),
        OpCode::Map => handle_map_op(shared_state.clone(), get_func_op_args(shared_state.clone(), &arg_fn_vals, call_context_id)?),
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
    let result: Vec<(ValueReference, FuncValLive)> = result_val_refs.iter()
        .zip(output_func_vals)
        .map(|(val_ref, func_val)| (val_ref.clone(), func_val))
        .collect();

    log_async(call_context_id, &format!("Operation ({}) executed successfully with result: {:?}", op.opcode, result[0].1.symbol));

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
            args
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

fn handle_map_op(shared_state: Arc<SharedCallState>, args: Vec<ValueReference>) -> ExecResult<Vec<ValueReference>> {
    if args.len() != 2 {
        return Err("Map operation requires 2 arguments".to_string());
    }

    let func = args[0].clone();
    let list = match shared_state.mmu.get_ref_value(&args[1]) {
        Ok(StoredData::ListStored(list)) => list,
        _ => return Err("Map operation requires a list as the second arg".to_string())
    };

    let (tx, rx) = mpsc::channel();
    let shared_tx = Arc::new(tx);

    for (i, item_ptr) in list.iter().enumerate() {
        let item_ref = value_ref_from_ptr(shared_state.mmu.clone(), item_ptr.clone())?;

        let shared_tx = shared_tx.clone();
        let shared_tx2 = shared_tx.clone();

        let output_callback = Arc::new(move |message: &NewValMessage| {
            let tx2 = shared_tx.clone();
            tx2.send(Ok((i, message.value.clone()))).unwrap();
        });

        let result_callback = Arc::new(move |result: ExecResult<()>| {
            let tx2 = shared_tx2.clone();

            if let Err(e) = result {
                tx2.send(Err(e)).unwrap();
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
            result_callback
        );
    }

    let unsorted_results: Mutex<Vec<(usize, ValueReference)>> = Mutex::new(Vec::with_capacity(list.len()));

    for result in rx.iter().take(list.len()) {
        match result {
            Ok((i, val)) => {
                let mut results = unsorted_results.lock().unwrap();
                results.push((i, val));
            },
            Err(msg) => return Err(format!("Error executing reduce function: {}", msg))
        }
    }

    // sort and return the results
    let mut results = unsorted_results.into_inner().unwrap();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    let results: Vec<ValueReference> = results.into_iter().map(|(_, val)| val).collect();
    let results: Vec<&ValueReference> = results.iter().collect();

    let store_op: StoreOp = StoreOp::StoreList(results);
    execute_store(shared_state.mmu.clone(), store_op)
}

// Executes a function synchronously
// fn execute_call_sync()