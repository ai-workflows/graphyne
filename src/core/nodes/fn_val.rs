use crate::core::data::stored::StoredData;

/// Represents a unique string that uniquely identifies a value within the scope of a function call.
pub type ValIdentifier = String;

#[derive(Debug)]
pub struct FunctionValueNode {
    /// The unique identifier for this "variable" within the scope of a function call.
    pub guid: ValIdentifier,

    /// Optional constant value that this variable is initialized to.
    pub constant: Option<StoredData>,
}

impl FunctionValueNode {
    pub fn new(guid: ValIdentifier) -> Self {
        Self {
            guid,
            constant: None,
        }
    }

    pub fn constant(guid: ValIdentifier, constant: StoredData) -> Self {
        Self {
            guid,
            constant: Some(constant),
        }
    }
}