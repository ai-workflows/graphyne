use std::sync::{Arc};
use crate::core::data::live::{FuncOpLive, FuncValLive};
use crate::core::{ExecResult};
use crate::core::data::functions::OpCode;
use crate::core::vm::functions::v2::shared::{CallContextId, SharedCallState, get_func_vals_from_ptrs};
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;

/// A worker responsible for executing

/// Executes an operation within the scope of a function call context.
/// Retrieves the arg values from the state, executes the operation, and returns the result values.
pub fn try_execute_fn_op<'a>(shared_state: Arc<SharedCallState<'a>>, op: &FuncOpLive, call_context_id: &CallContextId) -> ExecResult<Vec<(ValueReference<'a>, FuncValLive)>> {
    // get the func vals for the arguments
    let arg_fn_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(shared_state.vm.clone(), &op.input_vals) {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error getting input func vals for operation: {}", msg))
    };

    validate_op_inputs(&shared_state, &arg_fn_vals, call_context_id)?;

    let result_val_refs = handle_call_function_op(shared_state.clone(), &op.opcode, &arg_fn_vals, call_context_id)
        .map_err(|msg| format!("Execution of operation {} failed: {}", op.opcode, msg))?;

    if result_val_refs.len() != op.output_vals.len() {
        return Err(format!("Operation expected {} result values, but got {}", op.output_vals.len(), result_val_refs.len()));
    }

    // get the output func vals
    let output_func_vals: Vec<FuncValLive> = match get_func_vals_from_ptrs(shared_state.vm.clone(), &op.output_vals) {
        Ok(vals) => vals,
        Err(msg) => return Err(format!("Error getting output func vals for operation: {}", msg))
    };

    // match the output func vals with the result val refs
    let result: Vec<(ValueReference, FuncValLive)> = result_val_refs.iter()
        .zip(output_func_vals)
        .map(|(val_ref, func_val)| (val_ref.clone(), func_val))
        .collect();

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
pub fn handle_call_function_op<'a>(shared_state: Arc<SharedCallState<'a>>, op_code: &OpCode, args: &Vec<FuncValLive>, call_context_id: &CallContextId, ) -> ExecResult<Vec<ValueReference<'a>>> {
    let arg_values: Vec<ValueReference> = get_func_op_args(shared_state.clone(), args, call_context_id)?;
    let arg_values: Vec<&ValueReference> = arg_values.iter().collect();
    let op: Operation = op_code.to_operation(&arg_values);
    
    let res = shared_state.vm.execute_op(op);
    
    if let Err(msg) = res {
        return Err(format!("Operation execution failed: {}", msg));
    }
    
    let res = res.unwrap();
    
    let mut res2: Vec<ValueReference> = vec![];
    
    for val in res {
        let val2 = ValueReference::new(val.pointer, shared_state.vm.clone());
        
        res2.push(val);
    }

    Ok(res2)
}

/// Gets the arguments to a function's operation from the state manager
fn get_func_op_args<'a>(shared_state: Arc<SharedCallState<'a>>, args: &Vec<FuncValLive>, call_context_id: &CallContextId) -> ExecResult<Vec<ValueReference<'a>>> {
    args.iter()
        .map(move |arg_fn_val| {
            match shared_state.get_val(call_context_id, arg_fn_val) {
                Some(val) => Ok(val),
                None => Err("Arg value not found in state and not caught by validation".to_string())
            }
        })
        .collect()
}
