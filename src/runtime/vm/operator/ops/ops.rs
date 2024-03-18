use std::sync::Arc;
use crate::runtime::data::stored::StoredData;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
pub enum Operation {
    /// Get the type of a value
    TypeOf(Arc<StoredData>),
    
    /// Converts a value to an integer.
    AsInt(Arc<StoredData>),

    /// Converts a value to a float.
    AsFloat(Arc<StoredData>),

    /// Converts a value to a string.
    AsString(Arc<StoredData>),

    /// Converts a value to a boolean.
    AsBool(Arc<StoredData>),

    /// Converts a value to a pointer.
    AsPointer(Arc<StoredData>),

    /// Converts a value to a list.
    AsList(Arc<StoredData>),

    /// Converts a value to a dictionary.
    AsDictionary(Arc<StoredData>),

    /// Converts a value to a type.
    AsType(Arc<StoredData>),

    /// Returns the second value if the first value is true, otherwise returns the third value.
    If(Arc<StoredData>, Arc<StoredData>, Arc<StoredData>),

    /// Inverts a boolean value.
    Not(Arc<StoredData>),

    /// Returns a bool indicating whether both values are true.
    And(Arc<StoredData>, Arc<StoredData>),

    /// Returns a bool indicating whether either value is true.
    Or(Arc<StoredData>, Arc<StoredData>),

    /// Returns a bool indicating whether two values are equal.
    Equal(Arc<StoredData>, Arc<StoredData>),

    /// Returns true if the first value is less than the second value.
    LessThan(Arc<StoredData>, Arc<StoredData>),

    /// Returns true if the first value is greater than the second value.
    GreaterThan(Arc<StoredData>, Arc<StoredData>),

    /// Returns true if the value is null.
    IsNull(Arc<StoredData>),

    /// Gets the length of a collection.
    Length(Arc<StoredData>),

    /// Gets the value at a given index
    GetItem(Arc<StoredData>, Arc<StoredData>),

    /// Sets the value at a given index
    SetItem(Arc<StoredData>, Arc<StoredData>, Arc<StoredData>),

    /// Pushes a value onto a list
    Push(Arc<StoredData>, Arc<StoredData>),

    /// Removes a value from a list at a given index
    Remove(Arc<StoredData>, Arc<StoredData>),

    /// Adds two values together.
    Add(Arc<StoredData>, Arc<StoredData>),

    /// Subtracts two values.
    Sub(Arc<StoredData>, Arc<StoredData>),

    /// Multiplies two values.
    Mul(Arc<StoredData>, Arc<StoredData>),

    /// Divides two values.
    Div(Arc<StoredData>, Arc<StoredData>),

    /// Gets the remainder of two values.
    Mod(Arc<StoredData>, Arc<StoredData>),

    /// Raises a value to a power.
    Pow(Arc<StoredData>, Arc<StoredData>),

    /// Calls a function.
    Call(Arc<StoredData>, Vec<Arc<StoredData>>),

    /// Applies a function to each element of a list.
    Map(Arc<StoredData>, Arc<StoredData>),

    /// Applies a combining function to each element of a list, returning a single value.
    Reduce(Arc<StoredData>, Arc<StoredData>, Arc<StoredData>),

    /// Gets the items in a list that match a given predicate.
    Filter(Arc<StoredData>, Arc<StoredData>),

    /// Initializes an object of the given custom type using the given data.
    Init(Arc<StoredData>, Vec<Arc<StoredData>>),
    //
    // /// Casts an object to a different type.
    // Cast(Arc<StoredData>, Arc<StoredData>),
}