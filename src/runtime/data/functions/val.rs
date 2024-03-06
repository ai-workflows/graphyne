use crate::runtime::data::live::{PointerLive, StringLive};
use crate::runtime::Symbol;

pub type FuncValId = StringLive;

/// Represents a value that exists within the scope of a function call.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncVal {
    /// The local symbol for this value.
    pub symbol: Option<Symbol>,

    /// A globally unique identifier for this value.
    pub guid: FuncValId,

    /// A list of pointers to the func op nodes that depend on this value.
    pub dependents: Vec<PointerLive>,

    /// An optional constant value that this variable is initialized to.
    pub constant: Option<PointerLive>,
    
    /// Whether this value is a pointer to the function's class context.
    pub is_self: bool,
}