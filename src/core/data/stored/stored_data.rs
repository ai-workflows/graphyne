use crate::core::data::live::{FloatLive, IntLive};

/// Represents data that is currently being stored in memory.
/// This data must be converted to its live counterpart before it can be used.
#[derive(Debug)]
pub enum StoredData {
    IntStored(IntLive),
    FloatStored(FloatLive),
}

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