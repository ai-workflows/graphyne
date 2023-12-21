use std::collections::HashMap;
use crate::core::data::live::{BoolLive, FloatLive, IntLive, StringLive};
use crate::core::Symbol;

/// The types of constants that can be stored in a collection.
#[derive(Debug, Clone)]
pub enum CCData {
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Bool(BoolLive),
    List(Vec<CCData>),
    Dict(HashMap<Symbol, CCData>),
}

/// A constant value stored in a collection.
#[derive(Debug, Clone)]
pub struct CollectionConst {
    /// The constant's data.
    pub data: CCData,
}

impl From<IntLive> for CollectionConst {
    fn from(value: IntLive) -> Self {
        CollectionConst {
            data: CCData::Int(value),
        }
    }
}

impl From<IntLive> for CCData {
    fn from(value: IntLive) -> Self {
        CCData::Int(value)
    }
}

impl From<FloatLive> for CollectionConst {
    fn from(value: FloatLive) -> Self {
        CollectionConst {
            data: CCData::Float(value),
        }
    }
}

impl From<FloatLive> for CCData {
    fn from(value: FloatLive) -> Self {
        CCData::Float(value)
    }
}

impl From<StringLive> for CollectionConst {
    fn from(value: StringLive) -> Self {
        CollectionConst {
            data: CCData::String(value),
        }
    }
}

impl From<StringLive> for CCData {
    fn from(value: StringLive) -> Self {
        CCData::String(value)
    }
}

impl From<BoolLive> for CollectionConst {
    fn from(value: BoolLive) -> Self {
        CollectionConst {
            data: CCData::Bool(value),
        }
    }
}

impl From<BoolLive> for CCData {
    fn from(value: BoolLive) -> Self {
        CCData::Bool(value)
    }
}

impl From<Vec<CCData>> for CollectionConst {
    fn from(value: Vec<CCData>) -> Self {
        CollectionConst {
            data: CCData::List(value),
        }
    }
}

impl From<Vec<CCData>> for CCData {
    fn from(value: Vec<CCData>) -> Self {
        CCData::List(value)
    }
}

impl From<HashMap<Symbol, CCData>> for CollectionConst {
    fn from(value: HashMap<Symbol, CCData>) -> Self {
        CollectionConst {
            data: CCData::Dict(value),
        }
    }
}

impl From<HashMap<Symbol, CCData>> for CCData {
    fn from(value: HashMap<Symbol, CCData>) -> Self {
        CCData::Dict(value)
    }
}



