use std::fmt::Debug;
use std::marker::PhantomData;
use crate::core::data::live::{DictLive, FloatLive, IntLive, ListLive, PointerLive, StringLive};
use crate::core::data::live::live_data::BoolLive;
use crate::core::data::stored::StoredData;
use crate::core::gc::{GCPointer};

#[derive(PartialEq, Debug)]
pub enum GCObjectType {
    Integer,
    Float,
    String,
    Bool,
    Pointer,
    List,
    Dict
}

#[derive(Debug)]
pub enum GCObjectData {
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Bool(BoolLive),
    Pointer(PointerLive),
    List(ListLive),
    Dict(DictLive),
}

pub struct GCObject<T> {
    pub data_type: GCObjectType,
    pub data: GCObjectData,
    pub ref_count: usize,
    pub phantom: PhantomData<T>,
}

impl<T> Debug for GCObject<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCObject")
            .field("data_type", &self.data_type)
            .field("data", &self.data)
            .field("ref_count", &self.ref_count)
            .finish()
    }
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

    pub fn to_bool(&self) -> Result<BoolLive, &'static str> {
        if self.data_type == GCObjectType::Bool {
            match &self.data {
                GCObjectData::Bool(value) => Ok(*value),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_pointer(&self) -> Result<GCPointer<StoredData>, &'static str> {
        if self.data_type == GCObjectType::Pointer {
            match &self.data {
                GCObjectData::Pointer(value) => {
                    let cloned = value.clone_unsafe();
                    return Ok(cloned)
                }
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    /// Returns a reference to the pointer data that allows for mutation
    pub fn as_pointer(&mut self) -> Result<&mut GCPointer<StoredData>, &'static str> {
        if self.data_type == GCObjectType::Pointer {
            match &mut self.data {
                GCObjectData::Pointer(value) => {
                    return Ok(value)
                }
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_list(&self) -> Result<ListLive, &'static str> {
        if self.data_type == GCObjectType::List {
            match &self.data {
                GCObjectData::List(value) => Ok(value.iter().map(|ptr| ptr.clone_unsafe()).collect()),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn as_list(&mut self) -> Result<&mut ListLive, &'static str> {
        if self.data_type == GCObjectType::List {
            match &mut self.data {
                GCObjectData::List(value) => Ok(value),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_dict(&self) -> Result<DictLive, &'static str> {
        if self.data_type == GCObjectType::Dict {
            match &self.data {
                GCObjectData::Dict(value) => Ok(value.iter().map(|(key, ptr)| (key.clone(), ptr.clone_unsafe())).collect()),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn as_dict(&mut self) -> Result<&mut DictLive, &'static str> {
        if self.data_type == GCObjectType::Dict {
            match &mut self.data {
                GCObjectData::Dict(value) => Ok(value),
                _ => Err("Invalid data type"),
            }
        } else {
            Err("Invalid data type")
        }
    }
}