use crate::core::data::functions::op_code::OpCode;
use crate::core::data::live::{PointerLive, StringLive};

pub type FuncOpId = StringLive;

/// Represents an operation that is executed within a function.
#[derive(Debug, Clone)]
pub struct FuncOp {
    pub guid: FuncOpId,

    /// The opcode of the operation.
    pub opcode: OpCode,

    /// A list of pointers to the func value nodes that are used as inputs for this operation.
    pub input_vals: Vec<PointerLive>,

    /// A list of pointers to the func value nodes that are the output of this operation.
    pub output_vals: Vec<PointerLive>,
}