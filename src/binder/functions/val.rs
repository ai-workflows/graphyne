use crate::runtime::data::live::PointerLive;
use crate::runtime::data::stored::StoredData;
use crate::runtime::Symbol;

#[derive(Debug, Clone)]
pub struct FunctionValueNode {
    /// The symbol for this "variable" within the scope of a function call.
    pub symbol: Symbol,

    /// Optional constant value that this variable is initialized to.
    pub constant: Option<StoredData>,
}

impl FunctionValueNode {
    /// Creates a new function value node with the given symbol, indicating that it is a variable.
    pub fn var(symbol: Symbol) -> Self {
        Self {
            symbol,
            constant: None,
        }
    }

    /// Creates a new function value node with the given symbol, indicating that its value is a pre-defined constant.
    /// The constant value will be stored in memory at compile time, and referenced with a pointer.
    pub fn constant(symbol: Symbol, constant: StoredData) -> Self {
        Self {
            symbol,
            constant: Some(constant),
        }
    }

    /// Creates a new function value node with the given symbol, indicating that its value is an external constant.
    /// The passed pointer will be used to reference the constant value.
    pub fn external(symbol: Symbol, ptr: PointerLive) -> Self {
        Self {
            symbol,
            constant: Some(StoredData::PointerStored(ptr)),
        }
    }
}