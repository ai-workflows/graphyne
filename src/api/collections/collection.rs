use std::collections::HashMap;
use crate::api::collections::func::CollectionFunc;
use crate::api::collections::c_const::CollectionConst;
use crate::core::{Symbol, SymbolPath};

/// A grouping of functions, constants, sub-collections, and types (in the future).
#[derive(Debug, Clone)]
pub struct Collection {
    /// The collection's functions.
    pub functions: HashMap<Symbol, CollectionFunc>,

    /// The collection's constants.
    pub constants: HashMap<Symbol, CollectionConst>,

    /// The collection's sub-collections.
    pub collections: HashMap<Symbol, Collection>,

    /// Values in other collections (including collections themselves) that are referenced by this collection.
    /// The keys are the local symbols used to reference the values.
    /// The values are a symbol path from root to the value.
    pub imports: HashMap<Symbol, SymbolPath>,
}

impl Collection {
    /// Creates a new collection with the given functions, constants, and sub-collections.
    pub fn new(functions: HashMap<Symbol, CollectionFunc>, constants: HashMap<Symbol, CollectionConst>, collections: HashMap<Symbol, Collection>, imports: HashMap<Symbol, SymbolPath>) -> Self {
        Self {
            functions,
            constants,
            collections,
            imports,
        }
    }
}