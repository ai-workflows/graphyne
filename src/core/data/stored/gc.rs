use crate::core::data::live::{FloatLive, IntLive};
use crate::core::data::stored::StoredData;
use crate::core::gc::{GarbageCollectable, GCObject, GCObjectType};

impl GarbageCollectable for StoredData {
    fn from_gc_object(object: &GCObject) -> Option<Self> {
        match object.data_type {
            GCObjectType::Integer => object.to_int().ok().and_then(|int_data| StoredData::IntStored(int_data as IntLive).into()),
            GCObjectType::Float => object.to_float().ok().and_then(|float_data| StoredData::FloatStored(float_data as FloatLive).into()),
            GCObjectType::String => object.to_string().ok().and_then(|string_data| StoredData::StringStored(string_data).into()),
        }
    }

    fn to_gc_object(&self) -> GCObject {
        match self {
            StoredData::IntStored(int_live) => {
                let data = int_live.to_ne_bytes().to_vec();
                GCObject {
                    data_type: GCObjectType::Integer,
                    data,
                    ref_count: 0,
                }
            }
            StoredData::FloatStored(float_live) => {
                let data = float_live.to_ne_bytes().to_vec();
                GCObject {
                    data_type: GCObjectType::Float,
                    data,
                    ref_count: 0,
                }
            }
            StoredData::StringStored(string_live) => {
                let data = string_live.as_bytes().to_vec();
                GCObject {
                    data_type: GCObjectType::String,
                    data,
                    ref_count: 0,
                }
            }
        }
    }
}