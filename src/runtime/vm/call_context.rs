use std::sync::atomic::{AtomicUsize};
use std::sync::{Arc, OnceLock};
use crate::runtime::data::functions::func::FuncLive;
use crate::runtime::data::live::{PointerLive, StaticRefLive};
use crate::runtime::vm::outputs::OutputType;

pub struct CallContext {
    /// Static reference to the function that this call context is for.
    pub func_ref: StaticRefLive,

    /// A list of pointers to the assigned values for each of the function's variables.
    pub val_buffer: Vec<OnceLock<PointerLive>>,

    /// The count of unknown arg vals for each func op.
    pub unknown_arg_counts: Vec<AtomicUsize>,

    /// The types of the function's outputs, indicating how they should be handled.
    pub output_types: Vec<OutputType>,

    /// A buffer for child call contexts spawned by this call context.
    pub child_calls: Vec<OnceLock<Arc<CallContext>>>,
}

impl CallContext {
    pub fn new(
        func_ref: StaticRefLive,
        output_types: Vec<OutputType>,
    ) -> CallContext {
        let func = get_static_func(&func_ref);

        CallContext {
            func_ref: func_ref.clone(),
            val_buffer: func.values.iter().map(|_| OnceLock::new()).collect(),
            unknown_arg_counts: func.ops.iter().map(|op| op.input_vals.len().into()).collect(),
            output_types,
            child_calls: func.call_ops.iter().map(|_| OnceLock::new()).collect(),
        }
    }
}

pub fn get_static_func(func_ref: &StaticRefLive) -> &FuncLive {
    match func_ref.as_ref().get() {
        Some(v) => match v.stored_as_func() {
            Ok(v) => v,
            Err(e) => panic!("CallContext::get_func: {}", e)
        },
        None => panic!("CallContext::get_func: func_ref does not point to a FuncValV2::Func")
    }
}