use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

/// Live data types. These are interoperable with rust types and can be used to perform operations.
pub type IntLive = i64;
pub type FloatLive = f64;
pub type StringLive = String;


/// Represents data that is currently usable for performing operations.
#[allow(unused_variables)]
pub trait LiveData {
    /// Returns the "language" type of this data.
    fn type_tag(&self) -> Type;

    /// Returns the "language" type code of this data.
    fn type_code(&self) -> ExecResult<IntLive> {
        Ok(match self.type_tag() {
            Type::Integer => 0,
            Type::Float => 1,
            Type::String => 2,
        })
    }

    /// Operations return None if they are not Implemented

    /// Type conversions for this data. Converts this to another live data type.
    fn as_int(&self) -> Option<ExecResult<IntLive>> {None}
    fn as_float(&self) -> Option<ExecResult<FloatLive>> {None}
    fn as_string(&self) -> Option<ExecResult<StringLive>> {None}

    /// Operations for this data.
    /// Casts stored data args to the appropriate live data type and performs the operation.
    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {None}
}