use std::sync::{Arc, OnceLock};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Sender;
use crate::runtime::data::live::PointerLive;
use crate::runtime::vm::call_context::CallContext;

pub struct MapLink {
    /// A reference to the call context that the map op is in
    pub source_context: Arc<CallContext>,

    /// The index of the result val of the map op in the source context
    pub source_result_val: usize,

    /// A reference to the vector of pointers to each result val of the map op
    pub result_buffer: Arc<Vec<OnceLock<PointerLive>>>,

    /// The index of the result val in the result buffer
    pub result_idx: usize,

    /// A reference to the remaining count of the map op
    pub remaining_count: Arc<AtomicUsize>,
}

pub struct FilterLink {
    /// A reference to the call context that the filter op is in
    pub source_context: Arc<CallContext>,

    /// The index of the result val of the filter op in the source context
    pub source_result_val: usize,

    /// A reference to the vector of pointers to each result val of the filter op
    pub result_buffer: Arc<Vec<OnceLock<PointerLive>>>,

    /// The index of the result val in the result buffer
    pub result_idx: usize,

    /// A reference to the remaining count of the filter op
    pub remaining_count: Arc<AtomicUsize>,

    /// A pointer to the original list
    pub source_list: PointerLive
}

#[derive(Clone)]
pub struct ReduceLink {
    /// A reference to the call context that the reduce op is in
    pub source_context: Arc<CallContext>,

    /// The index of the result val of the reduce op in the source context
    pub source_result_val: usize,

    /// A reference to the list that the reduce op is reducing
    pub source_list: PointerLive,

    /// The index of the current value in the source list
    pub source_idx: usize,

    /// A pointer to the function that the reduce op is calling
    pub called_func: PointerLive,
}

pub enum OutputType {
    /// the value is a final result that should be broadcast to the user
    Final(usize, Sender<(usize, PointerLive)>),

    /// the value is linked to the output val (indexed by usize) of a call op in another context
    CrossCallLink(Arc<CallContext>, usize),

    // indicates that the value is a partial result of a map op in another context
    MapLink(MapLink),

    /// indicates that the value is a partial result of a filter op in another context
    FilterLink(FilterLink),

    /// indicates that the value is a partial result of a reduce op in another context
    ReduceLink(ReduceLink),
}