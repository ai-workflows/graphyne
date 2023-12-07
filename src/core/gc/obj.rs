use crate::core::data::live::{FloatLive, IntLive};

#[derive(PartialEq, Debug)]
pub enum GCObjectType {
    Integer,
    Float,
    String
}

#[derive(Debug)]
pub struct GCObject {
    pub data_type: GCObjectType,
    pub data: Vec<u8>,
    pub ref_count: usize,
}

impl GCObject {
    pub fn to_int(&self) -> Result<IntLive, &'static str> {
        if self.data_type == GCObjectType::Integer {
            if self.data.len() == 4 {
                Ok(i32::from_ne_bytes(self.data.clone().try_into().unwrap()) as IntLive)
            }
            else if self.data.len() == 8 {
                Ok(i64::from_ne_bytes(self.data.clone().try_into().unwrap()) as IntLive)
            }
            else {
                Err("Invalid byte length for integer")
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_float(&self) -> Result<FloatLive, &'static str> {
        if self.data_type == GCObjectType::Float {
            if self.data.len() == 4 {
                Ok(f32::from_ne_bytes(self.data.clone().try_into().unwrap()).clone() as FloatLive)
            }
            else if self.data.len() == 8 {
                Ok(f64::from_ne_bytes(self.data.clone().try_into().unwrap()) as FloatLive)
            }
            else {
                Err("Invalid byte length for float")
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_string(&self) -> Result<String, &'static str> {
        if self.data_type == GCObjectType::String {
            Ok(String::from_utf8(self.data.clone()).unwrap())
        } else {
            Err("Invalid data type")
        }
    }
}