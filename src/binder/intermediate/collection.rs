use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::binder::intermediate::func::CollectionFunc;
use crate::binder::intermediate::r#const::CollectionConst;
use crate::binder::intermediate::r#type::CustomTypeDef;
use crate::runtime::{Symbol, SymbolPath};

/// A grouping of functions, constants, sub-intermediate, and types (in the future).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// The collection's functions.
    pub functions: Option<HashMap<Symbol, CollectionFunc>>,

    /// The collection's constants.
    pub constants: Option<HashMap<Symbol, CollectionConst>>,

    /// The collection's sub-intermediate.
    pub collections: Option<HashMap<Symbol, Collection>>,
    
    /// The collection's types
    pub types: Option<HashMap<Symbol, CustomTypeDef>>,

    /// Values in other intermediate (including intermediate themselves) that are referenced by this collection.
    /// The keys are the local symbols used to reference the values.
    /// The values are a symbol path from root to the value.
    pub imports: Option<HashMap<Symbol, SymbolPath>>,
}

impl Collection {
    /// Creates a new collection with the given functions, constants, and sub-intermediate.
    pub fn new(
        functions: HashMap<Symbol, CollectionFunc>,
        constants: HashMap<Symbol, CollectionConst>,
        collections: HashMap<Symbol, Collection>,
        types: HashMap<Symbol, CustomTypeDef>,
        imports: HashMap<Symbol, SymbolPath>,
    ) -> Self {
        Self {
            functions: Some(functions),
            constants: Some(constants),
            collections: Some(collections),
            types: Some(types),
            imports: Some(imports),
        }
    }
}