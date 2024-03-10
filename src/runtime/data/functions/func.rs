use crate::runtime::{Symbol, SymbolPath};
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::live::PointerLive;

#[derive(Debug, Clone, PartialEq)]
pub struct FuncVal {
    /// The local symbol for this value.
    pub symbol: Symbol,

    /// The index of this value in the func
    pub index: usize,

    /// A list of indices to the func op nodes that depend on this value.
    pub dependents: Vec<usize>,

    // An optional constant value that this variable is initialized to.
    pub constant: Option<PointerLive>,

    // Flag to indicate that this value is an output of the function
    pub output_idx: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncOp {
    /// The index of this operation in the func
    pub index: usize,

    /// The opcode of the operation.
    pub opcode: OpCode,

    /// A list of indices to the func value nodes that are used as inputs for this operation.
    pub input_vals: Vec<usize>,

    /// A list of indices to the func value nodes that are the output of this operation.
    pub output_vals: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncLive {
    /// The static symbol path that this function is defined at.
    pub symbol_path: SymbolPath,

    /// List of the function's values
    pub values: Vec<FuncVal>,

    /// List of the function's operations
    pub ops: Vec<FuncOp>,

    /// A list of the indices of the func value nodes that args will be bound to when the function is called.
    pub input_vals: Vec<usize>,

    /// A list of the indices of the func value nodes that the function's return value will be bound to.
    pub output_vals: Vec<usize>,

    /// A list of the indices of the func value nodes that have a constant value.
    pub constant_vals: Vec<usize>,
}