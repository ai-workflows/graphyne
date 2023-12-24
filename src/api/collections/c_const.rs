use std::collections::HashMap;
use serde::{Serialize};
use crate::core::data::live::{BoolLive, FloatLive, IntLive, StringLive};
use crate::core::Symbol;

/// The types of constants that can be stored in a collection.
#[derive(Debug, Clone, Serialize)]
pub enum CCData {
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Bool(BoolLive),
    List(Vec<CCData>),
    Dict(HashMap<Symbol, CCData>),
}

/// A constant value stored in a collection.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionConst(pub CCData);

impl From<IntLive> for CollectionConst {
    fn from(value: IntLive) -> Self {
        CollectionConst(CCData::Int(value))
    }
}

impl From<IntLive> for CCData {
    fn from(value: IntLive) -> Self {
        CCData::Int(value)
    }
}

impl From<FloatLive> for CollectionConst {
    fn from(value: FloatLive) -> Self {
        CollectionConst(CCData::Float(value))
    }
}

impl From<FloatLive> for CCData {
    fn from(value: FloatLive) -> Self {
        CCData::Float(value)
    }
}

impl From<StringLive> for CollectionConst {
    fn from(value: StringLive) -> Self {
        CollectionConst(CCData::String(value))
    }
}

impl From<StringLive> for CCData {
    fn from(value: StringLive) -> Self {
        CCData::String(value)
    }
}

impl From<BoolLive> for CollectionConst {
    fn from(value: BoolLive) -> Self {
        CollectionConst(CCData::Bool(value))
    }
}

impl From<BoolLive> for CCData {
    fn from(value: BoolLive) -> Self {
        CCData::Bool(value)
    }
}

impl From<Vec<CCData>> for CollectionConst {
    fn from(value: Vec<CCData>) -> Self {
        CollectionConst(CCData::List(value))
    }
}

impl From<Vec<CCData>> for CCData {
    fn from(value: Vec<CCData>) -> Self {
        CCData::List(value)
    }
}

impl From<HashMap<Symbol, CCData>> for CollectionConst {
    fn from(value: HashMap<Symbol, CCData>) -> Self {
        CollectionConst(CCData::Dict(value))
    }
}

impl From<HashMap<Symbol, CCData>> for CCData {
    fn from(value: HashMap<Symbol, CCData>) -> Self {
        CCData::Dict(value)
    }
}

impl From<CollectionConst> for CCData {
    fn from(value: CollectionConst) -> Self {
        value.0
    }
}