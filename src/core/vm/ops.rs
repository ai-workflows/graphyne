use crate::core::data::stored::StoredData;
use crate::core::gc::GCPointer;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
pub enum Operation {
    /// Stores a literal value in memory.
    StoreInput(StoredData),

    /// Converts a value to an integer.
    AsInt(GCPointer<StoredData>),

    /// Converts a value to a float.
    AsFloat(GCPointer<StoredData>),

    /// Converts a value to a string.
    AsString(GCPointer<StoredData>),

    /// Converts a value to a pointer.
    AsPointer(GCPointer<StoredData>),

    /// Converts a value to a list.
    AsList(GCPointer<StoredData>),

    /// Adds two values together.
    Add(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Subtracts two values.
    Sub(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Multiplies two values.
    Mul(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Divides two values.
    Div(GCPointer<StoredData>, GCPointer<StoredData>),
}