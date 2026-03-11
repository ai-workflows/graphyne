use std::sync::Arc;
use std::sync::atomic::Ordering;
use rayon::ThreadPool;
use crate::runtime::data::functions::func::{FuncLive, FuncOp, FuncVal};
use crate::runtime::data::live::{PointerLive, StaticRefLive};
use crate::runtime::data::stored::StoredData::ListStored;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::call_context::{CallContext, get_static_func};
use crate::runtime::vm::executor;
use crate::runtime::vm::outputs::{FilterLink, MapLink, OutputType, ReduceLink};

/// Initializes a child function call, fills child_calls buffer in the parent context, and dispatches the constant values.
/// Note: all inputs should be handled individually as they arrive.
pub fn init_child_call(
    func_ref: &StaticRefLive,
    call_index: usize,
    parent_context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let parent_func: &FuncLive = get_static_func(&parent_context.func_ref);
    let call_op_index = match parent_func.call_ops.get(call_index) {
        Some(v) => v,
        None => panic!("CallContext::get_call_op: call_ops[{}] out of bounds", call_index),
    };
    let call_op = get_op(parent_func, *call_op_index);

    let output_types: Vec<OutputType> = call_op.output_vals.iter().map(|output_val_idx| {
        OutputType::CrossCallLink(parent_context.clone(), *output_val_idx)
    }).collect();

    let called_func: &FuncLive = get_static_func(func_ref);

    if output_types.len() != called_func.output_vals.len() {
        panic!("CallContext::create_call: output_types length does not match called_func.output_vals length");
    }

    let child_context: Arc<CallContext> = Arc::new(CallContext::new(
        func_ref.clone(),
        output_types,
        parent_context.runtime_error.clone(),
    ));

    dispatch_call_constants(called_func, child_context.clone(), static_state.clone(), worker_pool.clone());

    for (i, arg_val_index) in call_op.input_vals.iter().skip(1).enumerate() {
        if let Some(v) = parent_context.val_buffer[*arg_val_index].get() {
            set_val(
                child_context.clone(),
                called_func.input_vals[i],
                v.clone(),
                static_state.clone(),
                worker_pool.clone(),
            );
        }
    }

    match parent_context.child_calls[call_index].set(child_context.clone()) {
        Ok(_) => (),
        Err(_) => panic!("CallContext::create_call: child_calls[{}] is already initialized", call_index),
    };
}

/// Initializes an anonymous function call with known inputs and dispatches inputs/constants.
pub fn init_anonymous_call(
    func_ref: &StaticRefLive,
    inputs: &[PointerLive],
    output_types: Vec<OutputType>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>,
    runtime_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    let func: &FuncLive = get_static_func(func_ref);

    let context: Arc<CallContext> = Arc::new(CallContext::new(
        func_ref.clone(),
        output_types,
        runtime_error,
    ));

    dispatch_call_constants(func, context.clone(), static_state.clone(), worker_pool.clone());

    for (i, v) in inputs.iter().enumerate() {
        set_val(context.clone(), func.input_vals[i], v.clone(), static_state.clone(), worker_pool.clone());
    }
}

fn dispatch_call_constants(
    func: &FuncLive,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    for i in &func.constant_vals {
        let f_val = match func.values.get(*i) {
            Some(v) => v,
            None => panic!("CallContext::new: constant_vals[{}] out of bounds", i),
        };
        set_val(context.clone(), *i, f_val.constant.clone().unwrap(), static_state.clone(), worker_pool.clone());
    }
}

pub fn dispatch_call_args(
    fn_val: &FuncVal,
    val: PointerLive,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    for (child_call_idx, input_idx) in &fn_val.arg_for {
        match input_idx {
            0 => init_child_call(
                val.as_static_ref().expect("dispatch_call_args: called function is not a static ref"),
                *child_call_idx,
                context.clone(),
                static_state.clone(),
                worker_pool.clone(),
            ),
            _ => {
                let child_context = match get_child_call_opt(context.clone(), *child_call_idx) {
                    Some(v) => v,
                    None => continue,
                };

                let input_val_idx: usize = match get_static_func(&child_context.func_ref).input_vals.get(*input_idx - 1) {
                    Some(v) => *v,
                    None => panic!("dispatch_call_args: input_idx out of bounds"),
                };

                set_val(child_context.clone(), input_val_idx, val.clone(), static_state.clone(), worker_pool.clone());
            }
        }
    }
}

pub fn get_child_call_opt(context: Arc<CallContext>, call_index: usize) -> Option<Arc<CallContext>> {
    match context.child_calls.get(call_index) {
        Some(v) => v.get().cloned(),
        None => panic!("CallContext::get_child_call_opt: call_index out of bounds"),
    }
}

pub fn get_op(func: &FuncLive, index: usize) -> &FuncOp {
    match func.ops.get(index) {
        Some(v) => v,
        None => panic!("CallContext::get_op: index out of bounds"),
    }
}

pub fn get_val(context: Arc<CallContext>, index: usize) -> PointerLive {
    match context.val_buffer[index].get() {
        Some(v) => v.clone(),
        None => panic!("CallContext::get_val: val_buffer[{}] is not initialized", index),
    }
}

pub fn set_val(
    context: Arc<CallContext>,
    index: usize,
    val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    match context.val_buffer[index].set(val.clone()) {
        Ok(_) => (),
        Err(_) => panic!("CallContext::set_val: val_buffer[{}] is already initialized", index),
    };

    let f_val: &FuncVal = match get_static_func(&context.func_ref).values.get(index) {
        Some(v) => v,
        None => panic!("CallContext::set_val: index out of bounds"),
    };

    dispatch_call_args(f_val, val.clone(), context.clone(), static_state.clone(), worker_pool.clone());

    if let Some(output_idx) = f_val.output_idx {
        let output_type = match context.output_types.get(output_idx) {
            Some(v) => v,
            None => panic!("CallContext::set_val: output_types[{}] out of bounds", output_idx),
        };

        match output_type {
            OutputType::Final(output_idx, output_sender) => {
                output_sender.send((*output_idx, val)).unwrap();
            }
            OutputType::FinalError(_) => {}
            OutputType::CrossCallLink(ctx, output_index) =>
                set_val(ctx.clone(), *output_index, val, static_state.clone(), worker_pool.clone()),
            OutputType::MapLink(link) =>
                handle_map_link(link, val, static_state.clone(), worker_pool.clone()),
            OutputType::FilterLink(link) =>
                handle_filter_link(link, val, static_state.clone(), worker_pool.clone()),
            OutputType::ReduceLink(link) =>
                handle_reduce_link(link, val, static_state.clone(), worker_pool.clone()),
        }
    }

    for op_index in &f_val.dependents {
        context.unknown_arg_counts[*op_index].fetch_sub(1, Ordering::Relaxed);

        if context.unknown_arg_counts[*op_index].load(Ordering::Relaxed) == 0 {
            executor::dispatch_op(context.clone(), *op_index, static_state.clone(), worker_pool.clone());
        }
    }
}

fn handle_map_link(
    link: &MapLink,
    val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    link.result_buffer[link.result_idx].set(val).unwrap();
    let prev_count = link.remaining_count.fetch_sub(1, Ordering::Relaxed);

    if prev_count == 1 {
        let result: Vec<PointerLive> = link.result_buffer.iter()
            .map(|v| v.get().unwrap().clone())
            .collect();
        set_val(link.source_context.clone(),
                link.source_result_val,
                PointerLive::new(ListStored(result)),
                static_state.clone(),
                worker_pool.clone());
    }
}

fn handle_filter_link(
    link: &FilterLink,
    val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    link.result_buffer[link.result_idx].set(val).unwrap();
    let prev_count = link.remaining_count.fetch_sub(1, Ordering::Relaxed);

    if prev_count == 1 {
        let bool_results: Vec<bool> = link.result_buffer.iter()
            .map(|v| *v.get().unwrap().clone().stored_as_bool().unwrap())
            .collect();

        let result: Vec<PointerLive> = link.source_list.stored_as_list().unwrap().iter()
            .zip(bool_results.iter())
            .filter(|(_, b)| **b)
            .map(|(v, _)| v.clone())
            .collect();

        set_val(link.source_context.clone(),
                link.source_result_val,
                PointerLive::new(ListStored(result)),
                static_state.clone(),
                worker_pool.clone());
    }
}

fn handle_reduce_link(
    link: &ReduceLink,
    val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    if link.source_idx + 1 == link.source_list.stored_as_list().unwrap().len() {
        set_val(link.source_context.clone(),
                link.source_result_val,
                val,
                static_state.clone(),
                worker_pool.clone());
    }
    else {
        executor::dispatch_next_reduce(
            executor::ReduceDispatch {
                source_context: link.source_context.clone(),
                source_result_val: link.source_result_val,
                source_list: link.source_list.clone(),
                next_idx: link.source_idx + 1,
                called_func: link.called_func.clone(),
                last_val: val,
            },
            static_state.clone(),
            worker_pool.clone(),
        );
    }
}
