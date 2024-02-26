use std::sync::{Arc};
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::live::{FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::runtime::ExecResult;
use crate::runtime::vm::shared::{CallContextId, get_func_from_ptr, get_func_op_from_ptr, get_func_val_from_ptr, get_func_vals_from_ptrs, send_new_op, send_new_val, SharedCallState};
use crate::runtime::mmu::value_ref::ValueReference;

/// The orchestrator receives messages that new values are known, stores/links them, and determines which operations
/// need to be executed next. It then sends messages to the executor to execute these operations.

pub fn handle_new_value_v2(
    shared_state: Arc<SharedCallState>,
    call_context_id: &CallContextId,
    func_val: &FuncValLive,
    value: ValueReference
) -> ExecResult<()> {
    shared_state.log_async(call_context_id, &format!(
        "Orchestrating ops for new value: {}",
        func_val.symbol.clone().unwrap_or("(Unknown symbol)".to_string())
    ));

    // set the val in the state manager
    shared_state.set_val(call_context_id.clone(), func_val.clone(), value.clone());

    // get the dependent ops for the value
    let dependent_ops: Vec<FuncOpLive> = match func_val.dependents.iter()
        .map(|op_ptr| get_func_op_from_ptr(shared_state.mmu.clone(), op_ptr))
        .collect() {
        Ok(ops) => ops,
        Err(e) => return Err(format!("Error getting dependent ops: {}", e))
    };

    for dependent_op in dependent_ops {
        if dependent_op.opcode == OpCode::Call {
            match handle_call_op(shared_state.clone(), &dependent_op, call_context_id, func_val, value.clone()) {
                Ok(_) => {},
                Err(e) => return Err(format!("Error handling dependent call op: {}", e))
            }
        }
        else {
            match handle_normal_op(shared_state.clone(), &dependent_op, call_context_id) {
                Ok(_) => {},
                Err(e) => return Err(format!("Error handling dependent op: {}", e))
            }
        }
    }
    // if the val is an output we need to link it to the func val in the parent context
    // if it is the last output for this call context, we need to clean up
    if shared_state.is_output(call_context_id, func_val) {
        // get the output info
        let (parent_call_context, parent_output_fn_val) =
            shared_state.get_output_info(call_context_id, func_val)
            .ok_or(format!("Output info not found for call context: {}", call_context_id))?;

        // deregister the output
        shared_state.remove_output(call_context_id, func_val);

        // send the new value message to the parent context
        send_new_val(shared_state.clone(), parent_call_context.clone(), &parent_output_fn_val, value);

        // if this was the last output, the call context has been fully executed and can be cleaned up
        if shared_state.num_remaining_outputs(call_context_id) == 0 {
            shared_state.finalize_call_context(call_context_id);
        }
    }

    Ok(())
}

fn handle_call_op(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    call_context_id: &CallContextId,
    fn_val: &FuncValLive,
    value: ValueReference
) -> ExecResult<()> {
    // check if the value is the first arg (the function being called)
    let first_arg_fn_val = match op.input_vals.get(0) {
        Some(ptr) => match get_func_val_from_ptr(shared_state.mmu.clone(), ptr) {
            Ok(val) => val,
            Err(e) => return Err(format!("Error getting first arg val: {}", e))
        }
        None => return Err("Call op does not a first arg val (function being called)".to_string())
    };

    // if it is, handle the call
    if first_arg_fn_val.guid == fn_val.guid {
        return handle_func_call(shared_state, op, call_context_id, value);
    }

    // otherwise, this is an arg of the called function
    handle_call_arg(shared_state, op, call_context_id, &first_arg_fn_val)
}

fn handle_func_call<'a>(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    parent_call_context_id: &CallContextId,
    value: ValueReference
) -> ExecResult<()> {
    // get the value for the called function
    let called_fn = match get_func_from_ptr(shared_state.mmu.clone(), &value.pointer) {
        Ok(val) => val,
        Err(e) => return Err(format!("Error getting called function: {}", e))
    };

    // generate a random call context id
    let call_context_id = uuid::Uuid::new_v4().to_string();

    // add the call to the call cache
    shared_state.register_call(&parent_call_context_id, &op.guid, &call_context_id);

    // register the outputs of the called function with the output vals of the call op
    let op_outputs_fn_vals: Vec<FuncValLive> = get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.output_vals)?;
    let called_fn_output_vals: Vec<FuncValLive> = get_func_vals_from_ptrs(shared_state.mmu.clone(), &called_fn.output_vals)?;

    if op_outputs_fn_vals.len() != called_fn_output_vals.len() {
        return Err(format!("Number of outputs of operation does not match number of outputs for called function: {} != {}", op_outputs_fn_vals.len(), called_fn_output_vals.len()));
    }

    shared_state.register_outputs(&call_context_id, &op_outputs_fn_vals, parent_call_context_id, &called_fn_output_vals);

    // get the fn vals for the args in the parent context and the called function's context
    let arg_fn_vals: Vec<FuncValLive> = get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.input_vals[1..].to_vec())?;
    let called_fn_input_vals: Vec<FuncValLive> = get_func_vals_from_ptrs(shared_state.mmu.clone(), &called_fn.input_vals)?;

    if arg_fn_vals.len() != called_fn_input_vals.len() {
        return Err(format!("Number of args for operation does not match number of inputs for called function: {} != {}", arg_fn_vals.len(), called_fn_input_vals.len()));
    }

    // get the values for the args in the parent context
    let arg_vals: Vec<Option<ValueReference>> = shared_state.get_vals(parent_call_context_id, &arg_fn_vals);

    for (i, arg_val) in arg_vals.iter().enumerate() {
        match arg_val {
            Some(v_ref) => send_new_val(shared_state.clone(), call_context_id.clone(), &called_fn_input_vals[i], v_ref.clone()),
            None => return Err(format!("Arg val not found in parent call context: {}", parent_call_context_id))
        }
    }

    // handle the constants
    handle_called_fn_constants(shared_state, &call_context_id, &called_fn)?;

    Ok(())
}

// /// Handles an anonymous function call, such as for the main invocation of a program.
// pub fn handle_anonymous_fn_call(
//     shared_state: Arc<SharedCallState>,
//     call_context_id: &CallContextId,
//     called_fn: &FuncLive,
//     args: Vec<ValueReference>,
// ) -> ExecResult<()> {
//     // no need to register call or outputs, as this is an anonymous function
//     // send new value messages for the args
//     for (arg_ptr, arg_val) in called_fn.input_vals.iter().zip(args) {
//         let arg_fn_val = match get_func_val_from_ptr(shared_state.mmu.clone(), arg_ptr) {
//             Ok(val) => val,
//             Err(e) => {
//                 shared_state.handle_error(call_context_id, format!("Error getting arg val: {}", e));
//                 continue;
//             }
//         };
//         shared_state.send_new_val(call_context_id.clone(), arg_fn_val, arg_val);
//     }
//
//     // handle the constants
//     handle_called_fn_constants(shared_state, call_context_id, called_fn)?;
//
//     Ok(())
// }

pub fn handle_called_fn_constants(
    shared_state: Arc<SharedCallState>,
    call_context_id: &CallContextId,
    called_fn: &FuncLive
) -> ExecResult<()> {
    // loop over the constant values. Send a new value message for each
    let constant_fn_vals = get_func_vals_from_ptrs(shared_state.mmu.clone(), &called_fn.constant_vals)?;

    let mut constant_ptrs: Vec<PointerLive> = vec![];
    for constant_fn_val in &constant_fn_vals {
        let constant_ptr = match &constant_fn_val.constant {
            Some(ptr) => ptr,
            None => return Err(format!("Constant ptr not found for constant func val: {}", constant_fn_val.guid))
        };
        constant_ptrs.push(constant_ptr.clone());
    }

    let constant_vals = shared_state.value_refs_from_ptrs(constant_ptrs)?;

    for (constant_fn_val, constant_val) in constant_fn_vals.iter().zip(constant_vals) {
        send_new_val(shared_state.clone(), call_context_id.clone(), &constant_fn_val, constant_val);
    }

    Ok(())
}

fn handle_call_arg(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    call_context_id: &CallContextId,
    first_arg_fn_val: &FuncValLive
) -> ExecResult<()> {
    // check if the value of the first arg to the call op is known (the function being called)
    let first_arg_val = match shared_state.get_val(call_context_id, &first_arg_fn_val) {
        Some(val) => val,
        None => return Ok(()) // if it is not known, we will handle this arg later when the function is known
    };

    // get the function being called
    let called_fn = match get_func_from_ptr(shared_state.mmu.clone(), &first_arg_val.pointer) {
        Ok(val) => val,
        Err(e) => return Err(format!("Error getting called function: {}", e))
    };

    // get the call context id for the called function from the call cache
    let called_fn_context_id = match shared_state.get_child_call_context_id(call_context_id, &op.guid) {
        Some(id) => id.clone(),
        None => return Err("Could not find call context id for already called function. \
                Do you have an unused arg? The function may have already been garbage collected.".to_string())
    };

    // get the index of this val in the args of the dependent call op
    let mut arg_index = 0;
    for op_input_ptr in op.input_vals.iter() {
        let input_fn_val = get_func_val_from_ptr(shared_state.mmu.clone(), op_input_ptr)?;
        if input_fn_val.guid == first_arg_fn_val.guid {
            break;
        }
        arg_index += 1;
    }

    if arg_index == op.input_vals.len() {
        return Err("Could not find matching input for first arg".to_string());
    }

    // match to the input func val in the called function's context
    let matching_input_ptr = called_fn.input_vals.get(arg_index)
        .ok_or(format!("Could not find matching input for arg index: {}", arg_index))?;
    let matching_input_val: FuncValLive = get_func_val_from_ptr(shared_state.mmu.clone(), matching_input_ptr)?;

    // send a new value message for the matching input val
    send_new_val(shared_state.clone(), called_fn_context_id.clone(), &matching_input_val, first_arg_val.clone());

    Ok(())
}

fn handle_normal_op(
    shared_state: Arc<SharedCallState>,
    op: &FuncOpLive,
    call_context_id: &CallContextId,
) -> ExecResult<()> {
    // check if the op has already been executed (output vals are known)
    for output_fn_val in get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.output_vals)? {
        if shared_state.contains_val(call_context_id, &output_fn_val) {
            return Err(format!("Operation {} has already been executed.", op.opcode));
        }
    }

    // check if all input vals are known
    let input_fn_vals = get_func_vals_from_ptrs(shared_state.mmu.clone(), &op.input_vals)?;
    for input_fn_val in input_fn_vals {
        if !shared_state.contains_val(call_context_id, &input_fn_val) {
            // The operation will be executed later, when the last input value is known
            return Ok(());
        }
    }

    // // if this is a map op, handle it differently
    // if op.opcode == OpCode::Map {
    //     return handle_map_op(shared_state, op, call_context_id, input_fn_vals);
    // }

    // send a new op message to the executor
    send_new_op(shared_state.clone(), call_context_id.clone(), op.clone());

    Ok(())
}
