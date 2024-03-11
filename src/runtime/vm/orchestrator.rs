use std::sync::{Arc};
use std::sync::atomic::Ordering;
use rayon::ThreadPool;
use crate::runtime::data::functions::func::{FuncLive, FuncVal};
use crate::runtime::data::live::{PointerLive};
use crate::runtime::data::stored::StoredData::ListStored;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::call_context::{CallContext, get_static_func};
use crate::runtime::vm::executor;
use crate::runtime::vm::outputs::{FilterLink, MapLink, OutputType, ReduceLink};

/// Initializes the call of a function with the given inputs.
pub fn init_call(
    context: Arc<CallContext>,
    inputs: &[PointerLive],
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let func: &FuncLive = get_static_func(&context.func_ref);

    if inputs.len() != func.input_vals.len() {
        panic!("CallContext::new: inputs.len() != func.input_vals.len()");
    }

    // initialize the constant values
    for i in func.constant_vals.iter() {
        let f_val = match func.values.get(*i) {
            Some(v) => v,
            None => panic!("CallContext::new: constant_vals[{}] out of bounds", i)
        };
        set_val(context.clone(), *i, f_val.constant.clone().unwrap(), static_state.clone(), worker_pool.clone());
    }

    // initialize the input values
    for (i, v) in inputs.iter().enumerate() {
        let input_idx = func.input_vals[i];
        set_val(context.clone(), input_idx, v.clone(), static_state.clone(), worker_pool.clone());
    }
}

/// Gets a pointer to the assigned value for the given function variable.
pub fn get_val(context: Arc<CallContext>, index: usize) -> PointerLive {
    match context.val_buffer[index].get() {
        Some(v) => v.clone(),
        None => panic!("CallContext::get_val: val_buffer[{}] is not initialized", index)
    }
}

/// Assigns a value to the given function variable.
pub fn set_val(
    context: Arc<CallContext>,
    index: usize,
    val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    match context.val_buffer[index].set(val.clone()) {
        Ok(_) => (),
        Err(_) => panic!("CallContext::set_val: val_buffer[{}] is already initialized", index)
    };

    let f_val: &FuncVal = match get_static_func(&context.func_ref).values.get(index) {
        Some(v) => v,
        None => panic!("CallContext::set_val: index out of bounds")
    };

    // if this is an output value, handle it
    if let Some(output_idx) = f_val.output_idx {
        let output_type = match context.output_types.get(output_idx) {
            Some(v) => v,
            None => panic!("CallContext::set_val: output_types[{}] out of bounds", output_idx)
        };

        match output_type {
            OutputType::Final(output_idx, output_sender) => {
                output_sender.send((*output_idx, val)).unwrap();
            },
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

    // decrement the unknown arg count for each op that uses this value
    for op_index in f_val.dependents.iter() {
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

    // if this is the final value, convert the result buffer to a list and send it
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

    // if this is the final value, convert the result buffer to a list and send it
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
    // if this is the final value, set it in the source context
    if link.source_idx + 1 == link.source_list.stored_as_list().unwrap().len() {
        set_val(link.source_context.clone(),
                link.source_result_val,
                val,
                static_state.clone(),
                worker_pool.clone());
    }
    else {
        executor::dispatch_next_reduce(
            link.source_context.clone(),
            link.source_result_val,
            link.source_list.clone(),
            link.source_idx + 1,
            link.called_func.clone(),
            val,
            static_state.clone(),
            worker_pool.clone()
        );
    }
}