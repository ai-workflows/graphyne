/// Represents a unique string that uniquely identifies a value within the scope of a function call.
pub type ValIdentifier = String;

pub struct FunctionValueNode {
    /// The unique identifier for this "variable" within the scope of a function call.
    pub guid: ValIdentifier,
}

impl FunctionValueNode {
    pub fn new(guid: ValIdentifier) -> Self {
        Self {
            guid,
        }
    }
}