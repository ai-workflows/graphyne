use std::sync::Arc;
use crate::runtime::{ExecResult};
use crate::runtime::data::functions::v2::FuncV2;
use crate::runtime::data::live::{NullLive, IntLive, FloatLive, StringLive, PointerLive, ListLive, LiveData, DictLive, BoolLive, FuncLive, FuncValLive, FuncOpLive, StaticRefLive};
use crate::runtime::data::live::live_data::{ObjectLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;

/// New type wrapper for StoredData that implements LiveData using enum-based static_state dispatch.
struct LiveDispatch<'a>(&'a StoredData);

/// Add functions to convert stored data to live data.
impl StoredData {
    pub fn as_live(&self) -> impl LiveData + '_ {
        LiveDispatch(self)
    }
}

macro_rules! static_dispatch {
    { fn $name:tt ( $( $arg:tt : $argty:ty ),* ) -> $ret:ty } => {
        fn $name (&self, $( $arg : $argty ),* ) -> $ret {
            match self.0 {
                StoredData::NullStored => <NullLive as LiveData>::$name(&(), $( $arg ),* ),
                StoredData::IntStored(value) => <IntLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FloatStored(value) => <FloatLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::StringStored(value) => <StringLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::BoolStored(value) => <BoolLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::PointerStored(value) => <PointerLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::ListStored(value) => <ListLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::DictStored(value) => <DictLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FuncStored(value) => <FuncLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FuncValStored(value) => <FuncValLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FuncOpStored(value) => <FuncOpLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::TypeStored(value) => <TypeLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::ObjectStored(value) => <ObjectLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::StaticRefStored(value) => <StaticRefLive as LiveData>::$name(value, $( $arg ),* ),
                StoredData::FuncV2Stored(value) => <FuncV2 as LiveData>::$name(value, $( $arg ),* ),
            }
        }
    };
}


/// Implement LiveData for LiveDispatch.
impl LiveData for LiveDispatch<'_> {
    static_dispatch!{ fn type_of(static_state: Arc<StaticState>) -> Option<ExecResult<PointerLive>> }
    static_dispatch!{ fn as_int() -> Option<ExecResult<IntLive>> }
    static_dispatch!{ fn as_float() -> Option<ExecResult<FloatLive>> }
    static_dispatch!{ fn as_string() -> Option<ExecResult<StringLive>> }
    static_dispatch!{ fn as_bool() -> Option<ExecResult<BoolLive>> }
    static_dispatch!{ fn as_pointer() -> Option<ExecResult<PointerLive>> }
    static_dispatch!{ fn as_list() -> Option<ExecResult<ListLive>> }
    static_dispatch!{ fn as_dict() -> Option<ExecResult<DictLive>> }
    static_dispatch!{ fn as_func() -> Option<ExecResult<FuncLive>> }
    static_dispatch!{ fn as_func_val() -> Option<ExecResult<FuncValLive>> }
    static_dispatch!{ fn as_func_op() -> Option<ExecResult<FuncOpLive>> }
    static_dispatch!{ fn as_null() -> Option<ExecResult<NullLive>> }
    static_dispatch!{ fn as_type() -> Option<ExecResult<TypeLive>> }
    static_dispatch!{ fn as_object() -> Option<ExecResult<ObjectLive>> }
    static_dispatch!{ fn op_if(then: &StoredData, otherwise: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_not() -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_and(rhs: &StoredData) -> Option<ExecResult<StoredData>>}
    static_dispatch!{ fn op_or(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_eq(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_lt(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_gt(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn is_null() -> Option<ExecResult<BoolLive>> }
    static_dispatch!{ fn op_len() -> Option<ExecResult<IntLive>> }
    static_dispatch!{ fn op_get_item(index: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_set_item(index: &StoredData, value: PointerLive) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_push(value: PointerLive) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_remove(index: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_add(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_sub(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_mul(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_div(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_mod(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
    static_dispatch!{ fn op_pow(rhs: &StoredData) -> Option<ExecResult<StoredData>> }
}