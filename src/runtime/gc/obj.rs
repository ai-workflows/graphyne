use std::fmt::Debug;
use std::marker::PhantomData;
use crate::runtime::data::live::{DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, PointerLive, StringLive};
use crate::runtime::data::live::live_data::{BoolLive, ObjectLive, TypeLive};

#[derive(PartialEq, Debug, Clone)]
pub enum GCObjectType {
    Buffer,
    Integer,
    Float,
    String,
    Bool,
    Pointer,
    List,
    Dict,
    Func,
    FuncVal,
    FuncOp,
    Type,
    Object,
}

#[derive(Debug, Clone)]
pub enum GCObjectData {
    Null,
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Bool(BoolLive),
    Pointer(PointerLive),
    List(ListLive),
    Dict(DictLive),
    Func(FuncLive),
    FuncVal(FuncValLive),
    FuncOp(FuncOpLive),
    Type(TypeLive),
    Object(ObjectLive),
}

#[derive(Clone)]
pub struct GCObject<T> {
    pub data: T,
    pub ref_count: usize,
    pub phantom: PhantomData<T>,
}

impl<T> Debug for GCObject<T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCObject")
            .field("data", &self.data)
            .field("ref_count", &self.ref_count)
            .finish()
    }
}