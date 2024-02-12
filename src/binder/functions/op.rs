use serde::{Serialize};
use crate::runtime::data::functions::OpCode;
use crate::runtime::Symbol;

#[derive(Debug, Clone, Serialize)]
pub struct FunctionOpNode {
    /// The opcode that this function call represents.
    pub opcode: OpCode,
    
    /// References to the identifiers of the func value nodes that are used as inputs for this operation.
    pub input_vals: Vec<Symbol>,
    
    /// Reference to the identifiers of the func value nodes that are used as outputs for this operation.
    pub output_vals: Vec<Symbol>,
}

impl FunctionOpNode {
    pub fn new(opcode: OpCode, input_vals: Vec<Symbol>, output_val: Symbol) -> Self {
        if opcode == OpCode::Call {
            panic!("Cannot create a function op node with opcode Call. Use FunctionOpNode::call instead.");
        }

        Self {
            opcode,
            input_vals,
            output_vals: vec![output_val]
        }
    }

    pub fn call(func: Symbol, args: Vec<Symbol>, outputs: Vec<Symbol>) -> Self {
        // for calling a function, we use a special notation where the first input is the function itself.
        let input_vals = vec![func].into_iter().chain(args.into_iter()).collect();

        Self {
            opcode: OpCode::Call,
            input_vals,
            output_vals: outputs,
        }
    }
}