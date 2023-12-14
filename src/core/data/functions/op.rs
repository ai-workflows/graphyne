use crate::core::data::functions::op_code::OpCode;
use crate::core::data::live::PointerLive;

/// Represents an operation that is executed within a function.
#[derive(Debug, Clone)]
pub struct FuncOp {
    /// The opcode of the operation.
    pub opcode: OpCode,

    /// A list of pointers to the func value nodes that are used as inputs for this operation.
    pub input_vals: Vec<PointerLive>,

    /// A pointer to the func value node that is the output of this operation.
    pub output_val: PointerLive,
}