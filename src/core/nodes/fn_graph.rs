use std::collections::HashMap;
use crate::core::data::stored::StoredData;
use crate::core::nodes::{FunctionOpNode, FunctionValueNode};
use crate::core::nodes::fn_val::ValIdentifier;

/// Represents a graph of a function's values and operations, including what values are inputs and outputs.
#[derive(Debug)]
pub struct FunctionGraph {
    /// List of the function value nodes that exist within the scope of this function.
    pub values: Vec<FunctionValueNode>,

    /// List of the function op nodes that are executed within this function.
    pub ops: Vec<FunctionOpNode>,

    /// List of identifiers for the function value nodes that are used as inputs to this function.
    pub input_val_ids: Vec<ValIdentifier>,
    
    /// List of identifiers for the function value nodes that are outputs of this function.
    pub output_val_ids: Vec<ValIdentifier>,
}

impl FunctionGraph {
    pub fn new(values: Vec<FunctionValueNode>, ops: Vec<FunctionOpNode>, input_val_ids: Vec<ValIdentifier>, output_val_ids: Vec<ValIdentifier>) -> Self {
        Self {
            values,
            ops,
            input_val_ids,
            output_val_ids,
        }
    }
}