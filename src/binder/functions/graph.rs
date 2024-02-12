use crate::binder::functions::{FunctionOpNode, FunctionValueNode};
use crate::runtime::Symbol;

/// Represents a graph of a function's values and operations, including what values are inputs and outputs.
#[derive(Debug, Clone)]
pub struct FunctionGraph {
    /// List of the function value nodes that exist within the scope of this function.
    pub values: Vec<FunctionValueNode>,

    /// List of the function op nodes that are executed within this function.
    pub ops: Vec<FunctionOpNode>,

    /// List of identifiers for the function value nodes that are used as inputs to this function.
    pub input_vals: Vec<Symbol>,
    
    /// List of identifiers for the function value nodes that are outputs of this function.
    pub output_vals: Vec<Symbol>,
}

impl FunctionGraph {
    pub fn new(values: Vec<FunctionValueNode>, ops: Vec<FunctionOpNode>, input_vals: Vec<Symbol>, output_vals: Vec<Symbol>) -> Self {
        Self {
            values,
            ops,
            input_vals,
            output_vals,
        }
    }
}