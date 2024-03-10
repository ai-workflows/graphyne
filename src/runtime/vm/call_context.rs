use std::sync::atomic::{AtomicUsize};
use std::sync::{OnceLock};
use crate::runtime::data::functions::v2::{FuncV2};
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
}

impl CallContext {
    pub fn new(
        func_ref: StaticRefLive,
        output_types: Vec<OutputType>,
    ) -> CallContext {
        let mut res = CallContext {
            func_ref: func_ref.clone(),
            val_buffer: Vec::new(),
            unknown_arg_counts: Vec::new(),
            output_types,
        };

        let func = get_static_func(&func_ref);

        // generate val_buffer with the same length as the function's values
        res.val_buffer = func.values.iter().map(|_| OnceLock::new()).collect();

        // initialize the unknown arg counts
        for op in func.ops.iter() {
            res.unknown_arg_counts.push(op.input_vals.len().into());
        }

        res
    }
}

pub fn get_static_func(func_ref: &StaticRefLive) ->&FuncV2 {
    match func_ref.as_ref().get() {
        Some(v) => match v.stored_as_funcv2() {
            Ok(v) => v,
            Err(e) => panic!("CallContext::get_func: {}", e)
        },
        None => panic!("CallContext::get_func: func_ref does not point to a FuncValV2::Func")
    }
}