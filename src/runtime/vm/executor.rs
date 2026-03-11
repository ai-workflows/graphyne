use std::sync::{Arc, OnceLock};
use std::sync::atomic::AtomicUsize;
use rayon::ThreadPool;
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::functions::func::{FuncLive, FuncOp};
use crate::runtime::data::live::PointerLive;
use crate::runtime::data::stored::StoredData::ListStored;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::call_context::{get_static_func, CallContext};
use crate::runtime::vm::operator::operator::execute_op;
use crate::runtime::vm::operator::ops::Operation;
use crate::runtime::vm::orchestrator::{get_val, init_anonymous_call, set_val};
use crate::runtime::vm::outputs::{FilterLink, MapLink, OutputType, ReduceLink};

pub fn dispatch_op(
    context: Arc<CallContext>,
    op_idx: usize,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    worker_pool.clone().spawn(move || {
        let func: &FuncLive = get_static_func(&context.func_ref);
        let fn_op: &FuncOp = match func.ops.get(op_idx) {
            Some(o) => o,
            None => panic!("dispatch_op: op_idx out of bounds")
        };

        let inputs: Vec<PointerLive> = fn_op.input_vals.iter().map(|input_idx| {
            get_val(context.clone(), *input_idx).clone()
        }).collect();

        let op: Operation = fn_op.opcode.to_operation(&inputs);

        match fn_op.opcode {
            OpCode::Call => (),
            OpCode::Map => handle_map_op(fn_op, inputs, context.clone(), static_state, worker_pool),
            OpCode::Filter => handle_filter_op(fn_op, inputs, context.clone(), static_state, worker_pool),
            OpCode::Reduce => handle_reduce_op(fn_op, inputs, context.clone(), static_state, worker_pool),
            _ => handle_normal_op(fn_op, op, context.clone(), static_state, worker_pool)
        }
    });
}

pub fn handle_normal_op(
    fn_op: &FuncOp,
    op: Operation,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let outputs: Vec<PointerLive> = match execute_op(op, static_state.clone()) {
        Ok(v) => v,
        Err(e) => panic!("dispatch_op: execute_op error: {}", e)
    };

    for (i, v) in outputs.iter().enumerate() {
        set_val(context.clone(), fn_op.output_vals[i], v.clone(), static_state.clone(), worker_pool.clone());
    }
}

pub fn handle_map_op(
    fn_op: &FuncOp,
    inputs: Vec<PointerLive>,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let called_func_pointer: &PointerLive = match inputs.get(0) {
        Some(v) => v,
        None => panic!("dispatch_op: map op has no inputs")
    };

    let list_arg_ptr: &PointerLive = match inputs.get(1) {
        Some(v) => v,
        None => panic!("dispatch_op: map op has no list arg")
    };

    let list_arg: &Vec<PointerLive> = match list_arg_ptr.stored_as_list() {
        Ok(v) => v,
        Err(e) => panic!("dispatch_op: {}", e)
    };

    let result_val: usize = fn_op.output_vals[0];
    if list_arg.is_empty() {
        set_val(
            context,
            result_val,
            PointerLive::new(ListStored(Vec::new())),
            static_state,
            worker_pool,
        );
        return;
    }

    let result_buffer: Arc<Vec<OnceLock<PointerLive>>> = Arc::new(list_arg.iter().map(|_| OnceLock::new()).collect());
    let remaining_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(list_arg.len()));

    for (i, item) in list_arg.iter().enumerate() {
        let map_link: MapLink = MapLink {
            source_context: context.clone(),
            source_result_val: result_val,
            result_buffer: result_buffer.clone(),
            result_idx: i,
            remaining_count: remaining_count.clone()
        };

        let called_func_ref = match called_func_pointer.as_static_ref() {
            Ok(v) => v,
            Err(e) => panic!("dispatch_op: {}", e)
        };

        init_anonymous_call(
            called_func_ref,
            &[item.clone()],
            vec![OutputType::MapLink(map_link)],
            static_state.clone(),
            worker_pool.clone(),
        );
    }
}

pub fn handle_filter_op(
    fn_op: &FuncOp,
    inputs: Vec<PointerLive>,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let called_func_pointer: &PointerLive = match inputs.get(0) {
        Some(v) => v,
        None => panic!("dispatch_op: filter op has no inputs")
    };

    let list_arg_ptr: &PointerLive = match inputs.get(1) {
        Some(v) => v,
        None => panic!("dispatch_op: filter op has no list arg")
    };

    let list_arg: &Vec<PointerLive> = match list_arg_ptr.stored_as_list() {
        Ok(v) => v,
        Err(e) => panic!("dispatch_op: {}", e)
    };

    let result_val: usize = fn_op.output_vals[0];
    if list_arg.is_empty() {
        set_val(
            context,
            result_val,
            PointerLive::new(ListStored(Vec::new())),
            static_state,
            worker_pool,
        );
        return;
    }

    let result_buffer: Arc<Vec<OnceLock<PointerLive>>> = Arc::new(list_arg.iter().map(|_| OnceLock::new()).collect());
    let remaining_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(list_arg.len()));

    for (i, item) in list_arg.iter().enumerate() {
        let filter_link: FilterLink = FilterLink {
            source_context: context.clone(),
            source_result_val: result_val,
            result_buffer: result_buffer.clone(),
            result_idx: i,
            remaining_count: remaining_count.clone(),
            source_list: list_arg_ptr.clone()
        };

        let called_func_ref = match called_func_pointer.as_static_ref() {
            Ok(v) => v,
            Err(e) => panic!("dispatch_op: {}", e)
        };

        init_anonymous_call(
            called_func_ref,
            &[item.clone()],
            vec![OutputType::FilterLink(filter_link)],
            static_state.clone(),
            worker_pool.clone(),
        );
    }
}

pub fn dispatch_next_reduce(
    source_context: Arc<CallContext>,
    source_result_val: usize,
    source_list: PointerLive,
    next_idx: usize,
    called_func: PointerLive,
    last_val: PointerLive,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let new_link: ReduceLink = ReduceLink {
        source_context,
        source_result_val,
        source_list: source_list.clone(),
        source_idx: next_idx,
        called_func: called_func.clone()
    };

    let next_item: &PointerLive = match source_list.stored_as_list() {
        Ok(v) => match v.get(next_idx) {
            Some(v) => v,
            None => return
        },
        Err(e) => panic!("dispatch_next_reduce: {}", e)
    };

    let called_func_ref = match called_func.as_static_ref() {
        Ok(v) => v,
        Err(e) => panic!("dispatch_next_reduce: {}", e)
    };

    init_anonymous_call(
        called_func_ref,
        &[last_val, next_item.clone()],
        vec![OutputType::ReduceLink(new_link)],
        static_state.clone(),
        worker_pool.clone(),
    );
}

pub fn handle_reduce_op(
    fn_op: &FuncOp,
    inputs: Vec<PointerLive>,
    context: Arc<CallContext>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) {
    let called_func_pointer: &PointerLive = match inputs.get(0) {
        Some(v) => v,
        None => panic!("dispatch_op: reduce op has no inputs")
    };

    let list_arg_ptr: &PointerLive = match inputs.get(1) {
        Some(v) => v,
        None => panic!("dispatch_op: reduce op has no list arg")
    };

    let initial_val: &PointerLive = match inputs.get(2) {
        Some(v) => v,
        None => panic!("dispatch_op: reduce op has no initial value")
    };

    let result_val: usize = fn_op.output_vals[0];
    let source_list: &Vec<PointerLive> = match list_arg_ptr.stored_as_list() {
        Ok(v) => v,
        Err(e) => panic!("dispatch_op: {}", e)
    };

    if source_list.is_empty() {
        set_val(context, result_val, initial_val.clone(), static_state, worker_pool);
        return;
    }

    let source_context: Arc<CallContext> = context.clone();
    let source_result_val: usize = result_val;
    let source_list: PointerLive = list_arg_ptr.clone();
    let called_func: PointerLive = called_func_pointer.clone();

    dispatch_next_reduce(
        source_context,
        source_result_val,
        source_list,
        0,
        called_func,
        initial_val.clone(),
        static_state,
        worker_pool,
    );
}
