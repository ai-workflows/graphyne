use serde::{Deserialize, Serialize};
use crate::binder::intermediate::r#const::CCData;
use crate::runtime::data::functions::OpCode;
use crate::runtime::Symbol;

/// A function as it is represented in a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFunc {
    /// The function graph
    pub graph: CollectionFuncGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFuncGraph {
    /// List of the function value nodes that exist within the scope of this function.
    pub values: Vec<CFnValueNode>,

    /// List of the function op nodes that are executed within this function.
    pub ops: Vec<FunctionOpNode>,

    /// List of identifiers for the function value nodes that are used as inputs to this function.
    pub input_vals: Vec<Symbol>,

    /// List of identifiers for the function value nodes that are outputs of this function.
    pub output_vals: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CFnValueNode {
    /// The symbol for this "variable" within the scope of a function call.
    pub symbol: Symbol,

    /// Optional constant value that this variable is initialized to.
    pub constant: Option<CCData>,
}

impl CFnValueNode {
    /// Creates a new function value node with the given symbol, indicating that it is a variable.
    pub fn var(symbol: Symbol) -> Self {
        Self {
            symbol,
            constant: None,
        }
    }

    /// Creates a new function value node with the given symbol, indicating that its value is a pre-defined constant.
    /// The constant value will be stored in memory at compile time, and referenced with a pointer.
    pub fn constant(symbol: Symbol, constant: CCData) -> Self {
        Self {
            symbol,
            constant: Some(constant),
        }
    }

    // /// Creates a new function value node with the given symbol, indicating that its value is an external constant.
    // /// The passed pointer will be used to reference the constant value.
    // pub fn external(symbol: Symbol, ptr: PointerLive) -> Self {
    //     Self {
    //         symbol,
    //         constant: Some(StoredData::PointerStored(ptr)),
    //     }
    // }
}

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