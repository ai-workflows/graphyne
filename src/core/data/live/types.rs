use std::collections::HashMap;
use std::hash::Hash;
use crate::core::data::live::live_data::{ObjectLive, TypeLive};
use crate::core::data::live::{LiveData, PointerLive, StringLive};
use crate::core::{ExecResult, Symbol};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

/// Represents the "language" type of a piece of data.
/// Note: there may not be a one-to-one correspondence between this, rust-types, and stored-types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Integer,
    Float,
    String,
    Boolean,
    Pointer,
    List,
    Dictionary,
    Function,
    FunctionVal,
    FunctionOp,
    Null,
    Type,
    Dynamic,

    /// A custom type. Consists of a name, a guid, and a list of fields.
    Custom(Symbol, Symbol, Vec<(Symbol, PointerLive)>)
}

impl Type {
    pub fn get_name(&self) -> StringLive {
        match self {
            Type::Integer => StringLive::from("Integer"),
            Type::Float => StringLive::from("Float"),
            Type::String => StringLive::from("String"),
            Type::Boolean => StringLive::from("Boolean"),
            Type::Pointer => StringLive::from("Pointer"),
            Type::List => StringLive::from("List"),
            Type::Dictionary => StringLive::from("Dictionary"),
            Type::Function => StringLive::from("Function"),
            Type::FunctionVal => StringLive::from("FunctionVal"),
            Type::FunctionOp => StringLive::from("FunctionOp"),
            Type::Null => StringLive::from("Null"),
            Type::Type => StringLive::from("Type"),
            Type::Dynamic => StringLive::from("Dynamic"),
            Type::Custom(name, _, _) => name.clone(),
        }
    }
}

impl Hash for Type {
fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Type::Integer => 0.hash(state),
            Type::Float => 1.hash(state),
            Type::String => 2.hash(state),
            Type::Boolean => 3.hash(state),
            Type::Pointer => 4.hash(state),
            Type::List => 5.hash(state),
            Type::Dictionary => 6.hash(state),
            Type::Function => 7.hash(state),
            Type::FunctionVal => 8.hash(state),
            Type::FunctionOp => 9.hash(state),
            Type::Null => 10.hash(state),
            Type::Type => 11.hash(state),
            Type::Dynamic => 12.hash(state),
            Type::Custom(_, guid, _) => guid.hash(state),
        }
    }
}

impl LiveData for TypeLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Type, &type_map)
    }

    fn as_string(&self) -> Option<ExecResult<StringLive>> {
        Some(Ok(self.get_name()))
    }

    fn as_type(&self) -> Option<ExecResult<TypeLive>> {
        Some(Ok(self.clone()))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::TypeStored(rhs) => Some(Ok(StoredData::BoolStored(self == rhs))),
            _ => None,
        }
    }

}