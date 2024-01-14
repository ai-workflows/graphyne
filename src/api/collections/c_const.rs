use std::collections::HashMap;
use serde::{Serialize};
use crate::core::data::live::{BoolLive, FloatLive, IntLive, StringLive};
use crate::core::{ExecResult, Symbol};
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::store_op::StoreOp::{StoreBool, StoreFloat, StoreInt, StoreString};
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

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

impl VM {
    pub fn store_cc_data(&self, data: CCData) -> ExecResult<Vec<ValueReference>> {
        match data {
            CCData::Int(i) => self.execute_store(StoreInt(i)),
            CCData::Float(f) => self.execute_store(StoreFloat(f)),
            CCData::String(s) => self.execute_store(StoreString(s)),
            CCData::Bool(b) => self.execute_store(StoreBool(b)),
            CCData::List(l) => {
                let item_refs: Vec<Vec<ValueReference>> = l.iter().map(|c| self.store_cc_data(c.clone()).unwrap()).collect::<Vec<Vec<ValueReference>>>();
                let item_refs: Vec<ValueReference> = item_refs.into_iter().flatten().collect();

                self.execute_store(StoreOp::StoreList(item_refs.iter().collect()))
            }
            CCData::Dict(d) => {
                let item_refs: Vec<(String, Vec<ValueReference>)> = d.iter().map(|(k, v)| (k.clone(), self.store_cc_data(v.clone()).unwrap())).collect::<Vec<(String, Vec<ValueReference>)>>();
                let item_refs: HashMap<String, ValueReference> = item_refs.into_iter().map(|(k, v)| (k, v[0].clone())).collect();
                let item_refs: HashMap<String, &ValueReference> = item_refs.iter().map(|(k, v)| (k.clone(), v)).collect();

                self.execute_store(StoreOp::StoreDict(item_refs))

            }
        }
    }
}

