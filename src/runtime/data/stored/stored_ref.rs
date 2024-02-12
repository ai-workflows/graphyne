use crate::runtime::data::live::{BoolLive, DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, ObjectLive, PointerLive, StringLive, TypeLive};
use crate::runtime::data::stored::StoredData;

/// A version of stored data that contains a reference to a value rather than the value itself.
pub enum StoredRef<'a> {
    NullRef,
    IntRef(&'a IntLive),
    FloatRef(&'a FloatLive),
    StringRef(&'a StringLive),
    BoolRef(&'a BoolLive),
    PointerRef(&'a PointerLive),
    ListRef(&'a ListLive),  
    DictRef(&'a DictLive),
    FuncRef(&'a FuncLive),
    FuncValRef(&'a FuncValLive),
    FuncOpRef(&'a FuncOpLive),
    TypeRef(&'a TypeLive),
    ObjectRef(&'a ObjectLive),
}

// convert from stored ref to stored data
impl<'a> From<StoredRef<'a>> for StoredData {
    fn from(value: StoredRef) -> Self {
        match value {
            StoredRef::NullRef => StoredData::NullStored,
            StoredRef::IntRef(value) => StoredData::IntStored(value.clone()),
            StoredRef::FloatRef(value) => StoredData::FloatStored(value.clone()),
            StoredRef::StringRef(value) => StoredData::StringStored(value.clone()),
            StoredRef::BoolRef(value) => StoredData::BoolStored(value.clone()),
            StoredRef::PointerRef(value) => StoredData::PointerStored(value.clone()),
            StoredRef::ListRef(value) => StoredData::ListStored(value.clone()),
            StoredRef::DictRef(value) => StoredData::DictStored(value.clone()),
            StoredRef::FuncRef(value) => StoredData::FuncStored(value.clone()),
            StoredRef::FuncValRef(value) => StoredData::FuncValStored(value.clone()),
            StoredRef::FuncOpRef(value) => StoredData::FuncOpStored(value.clone()),
            StoredRef::TypeRef(value) => StoredData::TypeStored(value.clone()),
            StoredRef::ObjectRef(value) => StoredData::ObjectStored(value.clone()),
        }
    }
}

// get a ref to the stored data
impl<'a> From<&'a StoredData> for StoredRef<'a> {
    fn from(value: &'a StoredData) -> Self {
        match value {
            StoredData::NullStored => StoredRef::NullRef,
            StoredData::IntStored(value) => StoredRef::IntRef(value),
            StoredData::FloatStored(value) => StoredRef::FloatRef(value),
            StoredData::StringStored(value) => StoredRef::StringRef(value),
            StoredData::BoolStored(value) => StoredRef::BoolRef(value),
            StoredData::PointerStored(value) => StoredRef::PointerRef(value),
            StoredData::ListStored(value) => StoredRef::ListRef(value),
            StoredData::DictStored(value) => StoredRef::DictRef(value),
            StoredData::FuncStored(value) => StoredRef::FuncRef(value),
            StoredData::FuncValStored(value) => StoredRef::FuncValRef(value),
            StoredData::FuncOpStored(value) => StoredRef::FuncOpRef(value),
            StoredData::TypeStored(value) => StoredRef::TypeRef(value),
            StoredData::ObjectStored(value) => StoredRef::ObjectRef(value),
        }
    }
}