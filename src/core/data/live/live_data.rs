use std::collections::HashMap;
use crate::core::{ExecResult, Type};
use crate::core::data::functions::{FuncOp, FuncSig, FuncVal};
use crate::core::data::live::object::Object;
use crate::core::data::stored::StoredData;
use crate::core::gc::GCPointer;
use crate::core::vm::value_ref::ValueReference;

/// Live data types. These are interoperable with rust types and can be used to perform operations.
pub type NullLive = ();
pub type IntLive = i64;
pub type FloatLive = f64;
pub type StringLive = String;
pub type BoolLive = bool;
pub type PointerLive = GCPointer<StoredData>;
pub type ListLive = Vec<GCPointer<StoredData>>;
pub type DictLive = HashMap<StringLive, GCPointer<StoredData>>;
pub type TypeLive = Type;
pub type ObjectLive = Object;

// Live function types
pub type FuncLive = FuncSig;
pub type FuncValLive = FuncVal;
pub type FuncOpLive = FuncOp;


/// Represents data that is currently usable for performing operations.
#[allow(unused_variables)]
pub trait LiveData {
    /// Operations return None if they are not Implemented

    /// Returns a pointer to the type of this data. Requires passing a type map.
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {None}

    /// Type conversions for this data. Converts this to another live data type.
    fn as_int(&self) -> Option<ExecResult<IntLive>> {None}
    fn as_float(&self) -> Option<ExecResult<FloatLive>> {None}
    fn as_string(&self) -> Option<ExecResult<StringLive>> {None}
    fn as_bool(&self) -> Option<ExecResult<BoolLive>> {None}
    fn as_pointer(&self) -> Option<ExecResult<PointerLive>> {None}
    fn as_list(&self) -> Option<ExecResult<ListLive>> {None}
    fn as_dict(&self) -> Option<ExecResult<DictLive>> {None}
    fn as_func(&self) -> Option<ExecResult<FuncLive>> {None}
    fn as_func_val(&self) -> Option<ExecResult<FuncValLive>> {None}
    fn as_func_op(&self) -> Option<ExecResult<FuncOpLive>> {None}
    fn as_null(&self) -> Option<ExecResult<NullLive>> {None}
    fn as_type(&self) -> Option<ExecResult<TypeLive>> {None}
    fn as_object(&self) -> Option<ExecResult<ObjectLive>> {None}


    /// Boolean operations
    fn op_if(&self, then: &StoredData, otherwise: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_not(&self) -> Option<ExecResult<StoredData>> {None}
    fn op_and(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_or(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}

    /// Comparison operations
    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_lt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_gt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn is_null(&self) -> Option<ExecResult<BoolLive>> {None}


    /// Collection operations
    fn op_len(&self) -> Option<ExecResult<IntLive>> {None}
    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_set_item(&self, index: &StoredData, value: PointerLive) -> Option<ExecResult<StoredData>> {None}
    fn op_push(&self, value: PointerLive) -> Option<ExecResult<StoredData>> {None}
    fn op_remove(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {None}

    /// Arithmetic operations
    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_mod(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_pow(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
}