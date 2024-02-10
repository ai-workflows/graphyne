use std::sync::{Arc};
use crate::core::data::functions::OpCode;
use crate::core::data::live::{FuncLive, FuncOpLive, FuncValLive};
use crate::core::ExecResult;
use crate::core::vm::v2::shared::{CallContextId, get_func_from_ptr, get_func_op_from_ptr, get_func_val_from_ptr, get_func_vals_from_ptrs, log_async, SharedCallState};
use crate::core::vm::value_ref::ValueReference;

/// The orchestrator receives messages that new values are known, stores/links them, and determines which operations
/// need to be executed next. It then sends messages to the executor to execute these operations.

pub fn handle_new_value_v2(
    shared_state: Arc<SharedCallState>,
    call_context_id: &CallContextId,
    func_val: &FuncValLive,
    value: ValueReference
) -> ExecResult<()> {
    log_async(call_context_id, &format!(
        "Orchestrating ops for new value: {}",
        func_val.symbol.clone().unwrap_or("(Unknown symbol)".to_string())
    ));

    // if func_val.symbol == Some("two".to_string()) {
    //     let t = 2 + 2;
    // }
    //
    // if func_val.symbol == Some("num".to_string()) {
    //     let t = 3 + 3;
    // }

    if func_val.symbol == Some("doubled".to_string()) {
        let t = 3 + 3;
    }

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
        shared_state.send_new_val(parent_call_context, parent_output_fn_val, value);

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

    // register the function outputs in the output lookup
    for (i, output_ptr) in op.output_vals.iter().enumerate() {
        let parent_output_fn_val = match get_func_val_from_ptr(shared_state.mmu.clone(), output_ptr) {
            Ok(val) => val,
            Err(e) => return Err(format!("Error getting output val: {}", e))
        };

        // get the matching output fn val in the called function's context
        let called_fn_output_ptr = called_fn.output_vals.get(i)
            .ok_or(format!("Could not find matching output for output index: {}", i))?;
        let output_fn_val = get_func_val_from_ptr(shared_state.mmu.clone(), called_fn_output_ptr)?;

        shared_state.register_output(&call_context_id, &output_fn_val, parent_call_context_id, &parent_output_fn_val);
    }

    // loop over the arg values. If any are known, send a new value message for the matching input val in the called function's context
    for (i, arg_ptr) in op.input_vals.iter().enumerate() {
        // Skip the first element (the function being called)
        if i == 0 {
            continue;
        }

        // get the fn val for the arg in the parent context
        let arg_fn_val = match get_func_val_from_ptr(shared_state.mmu.clone(), arg_ptr) {
            Ok(val) => val,
            Err(e) => return Err(format!("Error getting arg val: {}", e))
        };

        // if the value is not known, we will handle this arg later
        if !shared_state.contains_val(parent_call_context_id, &arg_fn_val) {
            continue;
        }

        // get the value for the arg in the parent context
        let arg_val: ValueReference = match shared_state.get_val(parent_call_context_id, &arg_fn_val) {
            Some(val) => val,
            None => return Err(format!("Arg val not found in parent call context: {}", parent_call_context_id))
        };

        // get the to matching input fn val in the called function's context
        let called_fn_input_ptr = called_fn.input_vals.get(i - 1)
            .ok_or(format!("Could not find matching input for arg index: {}", i - 1))?;

        let call_input_fn_val = get_func_val_from_ptr(shared_state.mmu.clone(), called_fn_input_ptr)?;

        // send a new value message for the matching input val
        shared_state.send_new_val(call_context_id.clone(), call_input_fn_val, arg_val);
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
    for constant_ptr in called_fn.constant_vals.iter() {
        let constant_fn_val = match get_func_val_from_ptr(shared_state.mmu.clone(), constant_ptr) {
            Ok(val) => val,
            Err(e) => return Err(format!("Error getting constant val: {}", e))
        };

        let constant_ptr = match &constant_fn_val.constant {
            Some(ptr) => ptr,
            None => return Err(format!("Constant ptr not found for constant func val: {}", constant_fn_val.guid))
        };
        let constant_val = shared_state.value_ref_from_ptr(constant_ptr.clone())?;

        shared_state.send_new_val(call_context_id.clone(), constant_fn_val.clone(), constant_val);
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
    let matching_input_val = get_func_val_from_ptr(shared_state.mmu.clone(), matching_input_ptr)?;

    // send a new value message for the matching input val
    shared_state.send_new_val(called_fn_context_id.clone(), matching_input_val, first_arg_val.clone());

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

    // send a new op message to the executor
    shared_state.send_new_op(call_context_id.clone(), op.clone());

    Ok(())
}

// pub struct CallOrchestrator {
// }

// impl CallOrchestrator {




    // /// handles a newly calculated value by creating a meta func value for it and adding its
    // /// dependent ops to op queue.
    // pub fn handle_new_value(
    //     &self,
    //     vm: &VM,
    //     call_context_id: CallContextId,
    //     func_val: FuncValLive,
    //     value: ValueReference<'a>
    // ) -> ExecResult<()> {
    //     // create and add the meta func value
    //     let meta_val_id = self.add_meta_func_value(call_context_id.clone(), func_val.clone(), value)?;
    //
    //     // add the dependent ops to the op queue
    //     for dependent_op_ptr in func_val.dependents.iter() {
    //         let dependent_op = self.get_func_op_from_ptr(vm, dependent_op_ptr)?;
    //
    //         // check if this is a call op
    //         if dependent_op.opcode == OpCode::Call {
    //            // check if the call has already been registered
    //             let call_res = match self.call_cache.read().expect("Lock poisoned").get(&(call_context_id.clone(), dependent_op.guid.clone())) {
    //                 Some(call_res) => Some(call_res.clone()),
    //                 None => None
    //             };
    //
    //             if let Some(call_res) = call_res {
    //                 // if it has, we need to link the FuncVal for the matching arg to the MetaValue for the arg
    //                 // and add the dependent ops from the called func to the op queue
    //                 self.handle_arg_of_call_op(
    //                     vm,
    //                     &call_res.0,
    //                     &call_res.1,
    //                     &dependent_op,
    //                     &meta_val_id,
    //                     &func_val.guid)?;
    //             }
    //             else {
    //                 // if it hasn't we first need to check if the value is a function.
    //                 // if so, we can initialize the call and add the handle the inputs that are already known.
    //
    //                 // first get the value and check if it isn't a function
    //                 let called_fn: FuncLive = match vm.get_ref_value(&value) {
    //                     Ok(value_stored) => match value_stored.as_live().as_func() {
    //                         Some(Ok(val)) => val,
    //                         _ => continue // if the value is an arg, we must wait until the function is known
    //                     }
    //                     Err(e) => return Err(format!("Could not get newly known function in call op: {}", e))
    //                 };
    //
    //                 let first_arg_ptr = dependent_op.input_vals.get(0).ok_or(format!("Call op has no function pointer"))?;
    //                 let first_arg_fn: FuncLive = match vm.get_ptr_value(first_arg_ptr) {
    //                     Ok(StoredData::FuncStored(val)) => val,
    //                     Ok(_) => return Err(format!("First arg of call op does not point to a function: {:?}", first_arg_ptr)),
    //                     Err(e) => return Err(format!("Could not get function pointer: {}", e))
    //                 };
    //
    //                 // if the guids do not match, this is a function arg and should be handled later
    //                 if called_fn.guid != first_arg_fn.guid {
    //                     continue;
    //                 }
    //
    //                 // generate a new call context id
    //                 let new_call_context_id = uuid::Uuid::new_v4().to_string();
    //
    //                 // add the call to the cache
    //                 let mut call_cache = self.call_cache.write().expect("Lock poisoned");
    //                 call_cache.insert((call_context_id.clone(), dependent_op.guid.clone()), (new_call_context_id.clone(), value.pointer.clone()));
    //
    //                 // handle the constants
    //                 let constant_meta_ids = self.add_fn_constants(vm, &new_call_context_id, &called_fn)?;
    //
    //
    //             }
    //
    //             // we need to bind the arg to its func val in the called func so we can link it to the meta value
    //             // and add the dependent ops from the called func to the op queue
    //             // but what if we don't have the function value yet? We need a cache for the args until the func is known.
    //         }
    //
    //         let mut op_queue = self.op_queue.write().unwrap();
    //         op_queue.push((call_context_id.clone(), dependent_op));
    //     }
    //
    //
    //     Ok(())
    // }
    //
    // fn get_func_op_from_ptr(
    //     &self,
    //     vm: &VM,
    //     op_ptr: &PointerLive
    // ) ->ExecResult<FuncOpLive> {
    //     match vm.get_ptr_value(op_ptr) {
    //         Ok(value_stored) => match value_stored {
    //             StoredData::FuncOpStored(op) => Ok(op),
    //             _ => return Err(format!("Expected FuncOp, got: {:?}", value_stored))
    //         }
    //         Err(e) => return Err(format!("Error getting FuncOp: {}", e))
    //     }
    // }
    //
    // fn link_func_val_to_meta_value(&self, call_id: &CallContextId, input_val_guid: &FuncValId, meta_val_id: &MetaValueId) -> ExecResult<()> {
    //     let mut reverse_lookup = self.reverse_lookup.write().expect("Lock poisoned");
    //     reverse_lookup.entry(call_id.clone())
    //         .or_insert_with(HashMap::new)
    //         .insert(input_val_guid.clone(), meta_val_id.clone());
    //     Ok(())
    // }
    //
    // /// creates and adds a new MetaFuncVal for the given value
    // fn add_meta_func_value(
    //     &self,
    //     call_context_id: CallContextId,
    //     func_val: FuncValLive,
    //     value: ValueReference<'a>
    // ) -> ExecResult<MetaValueId> {
    //     let func_val_id = func_val.guid;
    //
    //     if self.is_func_val_known(&call_context_id, &func_val_id)? {
    //         return Err(format!("Value already calculated for call context, cannot be overwritten: {}, func val: {}", call_context_id, func_val_id));
    //     }
    //
    //     let meta_val_id = uuid::Uuid::new_v4().to_string();
    //     let meta_val = MetaFuncVal { value };
    //
    //     {
    //         let mut master_context = self.master_context.write().expect("Lock poisoned");
    //         master_context.insert(meta_val_id.clone(), meta_val);
    //     } // Lock is dropped here
    //
    //     self.link_func_val_to_meta_value(&call_context_id, &func_val_id, &meta_val_id)?;
    //
    //     Ok(meta_val_id)
    // }
    //
    // /// Handles a newly known value that is an argument to a call op (not the function) and the function is known.
    // fn handle_arg_of_call_op(
    //     &self,
    //     vm: &VM,
    //     call_id: &CallContextId, // id of the call context that this arg is part of
    //     called_fn_ptr: &PointerLive, // pointer to the function that is being called
    //     call_func_op: &FuncOpLive, // the call op node that this arg is part of
    //     meta_val_id: &MetaValueId, // the id of the meta value that this arg is part of
    //     func_val_id: &FuncValId, // the id of the func val that this arg is part of
    // ) -> ExecResult<()> {
    //     // retrieve the live data for the func
    //     let called_fn: FuncLive = self.get_func_from_ptr(vm, called_fn_ptr)?;
    //
    //     // get the index of the arg in the call op that is the func val of the new value
    //     let arg_index = call_func_op.input_vals.iter()
    //         .position(|&arg_ptr| matches!(
    //             vm.get_ptr_value(&arg_ptr).ok(),
    //             Some(StoredData::FuncValStored(val)) if val.guid == *func_val_id))
    //         .ok_or_else(|| format!("Could not find arg index for func val: {}", func_val_id))?;
    //
    //     // get the matching func val for the arg in the called func signature
    //     let matching_input_ptr = called_fn.input_vals.get(arg_index)
    //         .ok_or(format!("Could not find matching input for arg index: {}", arg_index))?;
    //
    //     let matching_input_val = vm.get_ptr_value(matching_input_ptr)
    //         .and_then(|data| match data {
    //             StoredData::FuncValStored(val) => Ok(val),
    //             _ => Err(format!("Matching input is not a FuncVal: {:?}", data))
    //         })?;
    //
    //     // link the func val to the existing meta value
    //     self.link_func_val_to_meta_value(call_id, &matching_input_val.guid, meta_val_id)?;
    //
    //     // add the dependent ops from the input func val to the op queue
    //     for dependent_op_ptr in matching_input_val.dependents.iter() {
    //         let dependent_op = self.get_func_op_from_ptr(vm, dependent_op_ptr)?;
    //         let mut op_queue = self.op_queue.write().expect("Lock poisoned");
    //         op_queue.push((call_id.clone(), dependent_op));
    //     }
    //
    //     Ok(())
    // }
    //
    // /// Adds the constant values of a function to the master context and creates meta values for them.
    // fn add_fn_constants(
    //     &self,
    //     vm: &VM,
    //     call_id: &CallContextId,
    //     func: &FuncLive
    // ) -> ExecResult<Vec<MetaValueId>> {
    //     func.constant_vals.iter().map(|constant_ptr| {
    //         let constant_fn_val = self.get_func_val_from_ptr(vm, constant_ptr)?;
    //         let constant_ref = match &constant_fn_val.constant {
    //             Some(ptr) => vm.value_ref_from_ptr(ptr.clone())?,
    //             None => return Err(format!("Constant value not found for constant func val: {}", constant_fn_val.guid))
    //         };
    //         self.add_meta_func_value(call_id.clone(), constant_fn_val, constant_ref)
    //     }).collect()
    // }
    //
    //
    //
    // fn is_func_val_known(
    //     &self,
    //     call_id: &CallContextId,
    //     func_val_id: &FuncValId
    // ) -> ExecResult<bool> {
    //     let reverse_lookup = self.reverse_lookup.read().expect("Lock poisoned");
    //     Ok(reverse_lookup.get(call_id).map_or(false, |call_context| call_context.contains_key(func_val_id)))
    // }
// }