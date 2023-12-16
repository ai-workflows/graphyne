use crate::core::data::functions::OpCode;
use crate::core::nodes::fn_val::ValIdentifier;

#[derive(Debug)]
pub struct FunctionOpNode {
    /// The opcode that this function call represents.
    pub opcode: OpCode,
    
    /// References to the identifiers of the func value nodes that are used as inputs for this operation.
    pub input_val_ids: Vec<ValIdentifier>,
    
    /// Reference to the identifier of the func value node that is the output of this operation.
    pub output_val_id: ValIdentifier,
}

impl FunctionOpNode {
    pub fn new(opcode: OpCode, input_val_ids: Vec<ValIdentifier>, output_val_id: ValIdentifier) -> Self {
        Self {
            opcode,
            input_val_ids,
            output_val_id,
        }
    }
}