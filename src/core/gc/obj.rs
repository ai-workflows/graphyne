#[derive(PartialEq)]
pub enum GCObjectType {
    Integer,
    Float,
    String
}

pub struct GCObject {
    pub data_type: GCObjectType,
    pub data: Vec<u8>,
    pub ref_count: usize,
}

impl GCObject {
    pub fn to_int(&self) -> Result<i32, &'static str> {
        if self.data_type == GCObjectType::Integer {
            if self.data.len() == 4 {
                Ok(i32::from_ne_bytes(self.data.clone().try_into().unwrap()))
            } else {
                Err("Invalid byte length for integer")
            }
        } else {
            Err("Invalid data type")
        }
    }

    pub fn to_float(&self) -> Result<f32, &'static str> {
        if self.data_type == GCObjectType::Float {
            if self.data.len() == 4 {
                Ok(f32::from_ne_bytes(self.data.clone().try_into().unwrap()).clone())
            } else {
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