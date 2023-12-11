use crate::core::data::stored::StoredData;
use crate::core::gc::GCPointer;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Operation {
    /// Stores a literal value in memory.
    StoreInput(StoredData),

    /// Converts a value to an integer.
    AsInt(GCPointer<StoredData>),

    /// Converts a value to a float.
    AsFloat(GCPointer<StoredData>),

    /// Converts a value to a string.
    AsString(GCPointer<StoredData>),

    /// Converts a value to a boolean.
    AsBool(GCPointer<StoredData>),

    /// Converts a value to a pointer.
    AsPointer(GCPointer<StoredData>),

    /// Converts a value to a list.
    AsList(GCPointer<StoredData>),

    /// Converts a value to a dictionary.
    AsDictionary(GCPointer<StoredData>),

    /// Returns the second value if the first value is true, otherwise returns the third value.
    If(GCPointer<StoredData>, GCPointer<StoredData>, GCPointer<StoredData>),

    /// Inverts a boolean value.
    Not(GCPointer<StoredData>),

    /// Returns a bool indicating whether both values are true.
    And(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Returns a bool indicating whether either value is true.
    Or(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Returns a bool indicating whether two values are equal.
    Equal(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Returns true if the first value is less than the second value.
    LessThan(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Returns true if the first value is greater than the second value.
    GreaterThan(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Gets the length of a collection.
    Length(GCPointer<StoredData>),

    /// Gets the value at a given index
    GetItem(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Sets the value at a given index
    SetItem(GCPointer<StoredData>, GCPointer<StoredData>, GCPointer<StoredData>),

    /// Pushes a value onto a list
    Push(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Removes a value from a list at a given index
    Remove(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Adds two values together.
    Add(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Subtracts two values.
    Sub(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Multiplies two values.
    Mul(GCPointer<StoredData>, GCPointer<StoredData>),

    /// Divides two values.
    Div(GCPointer<StoredData>, GCPointer<StoredData>),
}