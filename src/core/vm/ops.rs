use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Operation<'a> {
    /// Sets the value of a buffer
    SetBuffer(&'a ValueReference<'a>, StoredData),
    
    /// Converts a value to an integer.
    AsInt(&'a ValueReference<'a>),

    /// Converts a value to a float.
    AsFloat(&'a ValueReference<'a>),

    /// Converts a value to a string.
    AsString(&'a ValueReference<'a>),

    /// Converts a value to a boolean.
    AsBool(&'a ValueReference<'a>),

    /// Converts a value to a pointer.
    AsPointer(&'a ValueReference<'a>),

    /// Converts a value to a list.
    AsList(&'a ValueReference<'a>),

    /// Converts a value to a dictionary.
    AsDictionary(&'a ValueReference<'a>),

    /// Returns the second value if the first value is true, otherwise returns the third value.
    If(&'a ValueReference<'a>, &'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Inverts a boolean value.
    Not(&'a ValueReference<'a>),

    /// Returns a bool indicating whether both values are true.
    And(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Returns a bool indicating whether either value is true.
    Or(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Returns a bool indicating whether two values are equal.
    Equal(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Returns true if the first value is less than the second value.
    LessThan(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Returns true if the first value is greater than the second value.
    GreaterThan(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Returns true if the value is null.
    IsNull(&'a ValueReference<'a>),

    /// Gets the length of a collection.
    Length(&'a ValueReference<'a>),

    /// Gets the value at a given index
    GetItem(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Sets the value at a given index
    SetItem(&'a ValueReference<'a>, &'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Pushes a value onto a list
    Push(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Removes a value from a list at a given index
    Remove(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Adds two values together.
    Add(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Subtracts two values.
    Sub(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Multiplies two values.
    Mul(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Divides two values.
    Div(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Gets the remainder of two values.
    Mod(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Raises a value to a power.
    Pow(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Calls a function.
    Call(&'a ValueReference<'a>, Vec<&'a ValueReference<'a>>),

    /// Applies a function to each element of a list.
    Map(&'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Applies a combining function to each element of a list, returning a single value.
    Reduce(&'a ValueReference<'a>, &'a ValueReference<'a>, &'a ValueReference<'a>),

    /// Gets the items in a list that match a given predicate.
    Filter(&'a ValueReference<'a>, &'a ValueReference<'a>),
}