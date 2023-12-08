use std::marker::PhantomData;
use crate::core::data::live::{FloatLive, IntLive, StringLive};
use crate::core::data::stored::StoredData;
use crate::core::gc::{GCPointer};

#[derive(PartialEq, Debug)]
pub enum GCObjectType {
    Integer,
    Float,
    String,
    Pointer,
    // List,
}

#[derive(Debug)]
pub enum GCObjectData {
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Pointer(GCPointer<StoredData>),
}

#[derive(Debug)]
pub struct GCObject<T> {
    pub data_type: GCObjectType,
    pub data: GCObjectData,
    pub ref_count: usize,
    pub phantom: PhantomData<T>,
}

impl<T> GCObject<T> {
    pub fn to_int(&self) -> Result<IntLive, &'static str> {
        if self.data_type == GCObjectType::Integer {
            match &self.data {
                GCObjectData::Int(value) => Ok(*value),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_float(&self) -> Result<FloatLive, &'static str> {
        if self.data_type == GCObjectType::Float {
            match &self.data {
                GCObjectData::Float(value) => Ok(*value),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_string(&self) -> Result<StringLive, &'static str> {
        if self.data_type == GCObjectType::String {
            match &self.data {
                GCObjectData::String(value) => Ok(value.clone()),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_pointer(&self) -> Result<GCPointer<StoredData>, &'static str> {
        if self.data_type == GCObjectType::Pointer {
            match &self.data {
                GCObjectData::Pointer(value) => Ok(value.clone()),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }
}