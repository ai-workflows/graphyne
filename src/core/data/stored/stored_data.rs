use crate::core::data::live::{FloatLive, IntLive, StringLive, ListLive, PointerLive, DictLive, FuncLive, FuncValLive, FuncOpLive};
use crate::core::data::live::live_data::{BoolLive, ObjectLive, TypeLive};
use crate::core::gc::GCPointer;

/// Represents data that is currently being stored in memory.
/// This data must be converted to its live counterpart before it can be used.
#[derive(Debug, Clone)]
pub enum StoredData {
    NullStored,
    IntStored(IntLive),
    FloatStored(FloatLive),
    StringStored(StringLive),
    BoolStored(BoolLive),
    PointerStored(PointerLive),
    ListStored(ListLive),
    DictStored(DictLive),
    FuncStored(FuncLive),
    FuncValStored(FuncValLive),
    FuncOpStored(FuncOpLive),
    TypeStored(TypeLive),
    ObjectStored(ObjectLive),
}

// Convert from live data to stored data.
impl From<IntLive> for StoredData {
    fn from(value: IntLive) -> Self {
        StoredData::IntStored(value)
    }
}

impl From<FloatLive> for StoredData {
    fn from(value: FloatLive) -> Self {
        StoredData::FloatStored(value)
    }
}

impl From<StringLive> for StoredData {
    fn from(value: StringLive) -> Self {
        StoredData::StringStored(value)
    }
}

impl From<BoolLive> for StoredData {
    fn from(value: BoolLive) -> Self {
        StoredData::BoolStored(value)
    }
}

impl From<ListLive> for StoredData {
    fn from(value: ListLive) -> Self {
        StoredData::ListStored(value)
    }
}

impl From<GCPointer<StoredData>> for StoredData {
    fn from(value: GCPointer<StoredData>) -> Self {
        StoredData::PointerStored(value)
    }
}

impl From<DictLive> for StoredData {
    fn from(value: DictLive) -> Self {
        StoredData::DictStored(value)
    }
}

impl From<FuncLive> for StoredData {
    fn from(value: FuncLive) -> Self {
        StoredData::FuncStored(value)
    }
}

impl From<FuncValLive> for StoredData {
    fn from(value: FuncValLive) -> Self {
        StoredData::FuncValStored(value)
    }
}

impl From<FuncOpLive> for StoredData {
    fn from(value: FuncOpLive) -> Self {
        StoredData::FuncOpStored(value)
    }
}