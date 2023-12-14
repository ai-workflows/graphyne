use crate::core::data::live::{PointerLive, StringLive};

/// Represents a value that exists within the scope of a function call.
#[derive(Debug, Clone)]
pub struct FuncVal {
    /// A globally unique identifier for this value.
    pub guid: StringLive,

    /// A list of pointers to the func op nodes that depend on this value.
    pub dependents: Vec<PointerLive>,
}