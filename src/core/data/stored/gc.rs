use std::marker::PhantomData;
use crate::core::data::live::{FloatLive, IntLive};
use crate::core::data::stored::StoredData;
use crate::core::gc::{GarbageCollectable, GCObject, GCObjectData, GCObjectType};

impl GarbageCollectable<StoredData> for StoredData {
    fn from_gc_object(object: &GCObject<StoredData>) -> Option<Self> {
        match object.data_type {
            GCObjectType::Integer => object.to_int().ok().and_then(|int_data| StoredData::IntStored(int_data as IntLive).into()),
            GCObjectType::Float => object.to_float().ok().and_then(|float_data| StoredData::FloatStored(float_data as FloatLive).into()),
            GCObjectType::String => object.to_string().ok().and_then(|string_data| StoredData::StringStored(string_data).into()),
            GCObjectType::Bool => object.to_bool().ok().and_then(|bool_data| StoredData::BoolStored(bool_data).into()),
            GCObjectType::Pointer => object.to_pointer().ok().and_then(|pointer_data| StoredData::PointerStored(pointer_data).into()),
            GCObjectType::List => object.to_list().ok().and_then(|list_data| StoredData::ListStored(list_data).into()),
            GCObjectType::Dict => object.to_dict().ok().and_then(|dict_data| StoredData::DictStored(dict_data).into()),
        }
    }

    fn to_gc_object(&self) -> GCObject<StoredData> {
        match self {
            StoredData::IntStored(int_live) => {
                let data = GCObjectData::Int(*int_live);
                GCObject {
                    data_type: GCObjectType::Integer,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::FloatStored(float_live) => {
                let data = GCObjectData::Float(*float_live);
                GCObject {
                    data_type: GCObjectType::Float,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::StringStored(string_live) => {
                let data = GCObjectData::String(string_live.clone());
                GCObject {
                    data_type: GCObjectType::String,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::BoolStored(bool_live) => {
                let data = GCObjectData::Bool(*bool_live);
                GCObject {
                    data_type: GCObjectType::Bool,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::PointerStored(gc_pointer) => {
                let data = GCObjectData::Pointer(gc_pointer.clone());
                GCObject {
                    data_type: GCObjectType::Pointer,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::ListStored(list_live) => {
                let data = GCObjectData::List(list_live.clone());
                GCObject {
                    data_type: GCObjectType::List,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::DictStored(dict_live) => {
                let data = GCObjectData::Dict(dict_live.clone());
                GCObject {
                    data_type: GCObjectType::Dict,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
        }
    }
}