use crate::api::functions::FunctionGraph;

/// A function as it is represented in a collection.
#[derive(Debug, Clone)]
pub struct CollectionFunc {
    /// The function graph
    pub graph: FunctionGraph,
}