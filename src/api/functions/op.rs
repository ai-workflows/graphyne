use crate::core::data::functions::OpCode;
use crate::core::Symbol;

#[derive(Debug, Clone)]
pub struct FunctionOpNode {
    /// The opcode that this function call represents.
    pub opcode: OpCode,
    
    /// References to the identifiers of the func value nodes that are used as inputs for this operation.
    pub input_vals: Vec<Symbol>,
    
    /// Reference to the identifier of the func value node that is the output of this operation.
    pub output_val: Symbol,
}

impl FunctionOpNode {
    pub fn new(opcode: OpCode, input_vals: Vec<Symbol>, output_val: Symbol) -> Self {
        Self {
            opcode,
            input_vals,
            output_val,
        }
    }
}