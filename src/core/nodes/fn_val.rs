use crate::core::data::live::PointerLive;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

/// Represents a unique string that uniquely identifies a value within the scope of a function call.
pub type ValIdentifier = String;

#[derive(Debug)]
pub struct FunctionValueNode {
    /// The unique identifier for this "variable" within the scope of a function call.
    pub guid: ValIdentifier,

    /// Optional constant value that this variable is initialized to.
    pub constant: Option<StoredData>,

    /// Optional reference to an external value that this variable is initialized to.
    pub external: Option<PointerLive>,
}

impl FunctionValueNode {
    pub fn new(guid: ValIdentifier) -> Self {
        Self {
            guid,
            constant: None,
            external: None,
        }
    }

    pub fn constant(guid: ValIdentifier, constant: StoredData) -> Self {
        Self {
            guid,
            constant: Some(constant),
            external: None,
        }
    }

    pub fn external(guid: ValIdentifier, external: PointerLive) -> Self {
        Self {
            guid,
            constant: None,
            external: Some(external),
        }
    }
}