use crate::core::data::live::PointerLive;

/// Represents the signature of a function.
#[derive(Debug, Clone)]
pub struct FuncSig {
    /// A list of pointers to the func value nodes that args will be binded to when the function is called.
    pub input_vals: Vec<PointerLive>,

    /// A list of pointers to the func value nodes that the function will return when it is called.
    pub output_vals: Vec<PointerLive>
}