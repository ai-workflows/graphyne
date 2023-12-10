use crate::core::{ExecResult, Type};
use crate::core::data::live::{IntLive, FloatLive, StringLive, PointerLive, ListLive, LiveData};
use crate::core::data::stored::StoredData;

/// New type wrapper for StoredData that implements LiveData using enum-based static dispatch.
struct LiveDispatch<'a>(&'a StoredData);

/// Add functions to convert stored data to live data.
impl StoredData {
    pub fn as_live(&self) -> impl LiveData + '_ {
        LiveDispatch(self)
    }

    pub fn type_tag(&self) -> Type {
        self.as_live().type_tag()
    }

    pub fn type_code(&self) -> ExecResult<IntLive> {
        self.as_live().type_code()
    }
}

macro_rules! static_dispatch {
    { fn $name:tt ( $( $arg:tt : $argty:ty ),* ) -> $ret:ty } => {
        fn $name (&self, $( $arg : $argty ),* ) -> $ret {
            match self.0 {
                StoredData::IntStored(value) => <IntLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FloatStored(value) => <FloatLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::StringStored(value) => <StringLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::PointerStored(value) => <PointerLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::ListStored(value) => <ListLive as LiveData>::$name(value, $( $arg ),* ),
            }
        }
    };
}


/// Implement LiveData for LiveDispatch.
impl LiveData for LiveDispatch<'_> {
    static_dispatch!{ fn type_tag() -> Type }
    static_dispatch!{ fn type_code() -> ExecResult<IntLive> }
    static_dispatch!{ fn as_int() -> Option<ExecResult<IntLive>> }
    static_dispatch!{ fn as_float() -> Option<ExecResult<FloatLive>> }
    static_dispatch!{ fn as_string() -> Option<ExecResult<StringLive>> }
    static_dispatch!{ fn as_pointer() -> Option<ExecResult<PointerLive>> }
    static_dispatch!{ fn as_list() -> Option<ExecResult<ListLive>> }
    static_dispatch!{ fn op_add(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_sub(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_mul(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_div(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
}