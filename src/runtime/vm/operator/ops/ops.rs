use crate::runtime::data::stored::StoredData;
use crate::runtime::mmu::value_ref::ValueReference;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
pub enum Operation<'a> {
    /// Sets the value of a buffer
    SetBuffer(&'a ValueReference, StoredData),

    /// Get the type of a value
    TypeOf(&'a ValueReference),
    
    /// Converts a value to an integer.
    AsInt(&'a ValueReference),

    /// Converts a value to a float.
    AsFloat(&'a ValueReference),

    /// Converts a value to a string.
    AsString(&'a ValueReference),

    /// Converts a value to a boolean.
    AsBool(&'a ValueReference),

    /// Converts a value to a pointer.
    AsPointer(&'a ValueReference),

    /// Converts a value to a list.
    AsList(&'a ValueReference),

    /// Converts a value to a dictionary.
    AsDictionary(&'a ValueReference),

    /// Converts a value to a type.
    AsType(&'a ValueReference),

    /// Returns the second value if the first value is true, otherwise returns the third value.
    If(&'a ValueReference, &'a ValueReference, &'a ValueReference),

    /// Inverts a boolean value.
    Not(&'a ValueReference),

    /// Returns a bool indicating whether both values are true.
    And(&'a ValueReference, &'a ValueReference),

    /// Returns a bool indicating whether either value is true.
    Or(&'a ValueReference, &'a ValueReference),

    /// Returns a bool indicating whether two values are equal.
    Equal(&'a ValueReference, &'a ValueReference),

    /// Returns true if the first value is less than the second value.
    LessThan(&'a ValueReference, &'a ValueReference),

    /// Returns true if the first value is greater than the second value.
    GreaterThan(&'a ValueReference, &'a ValueReference),

    /// Returns true if the value is null.
    IsNull(&'a ValueReference),

    /// Gets the length of a collection.
    Length(&'a ValueReference),

    /// Gets the value at a given index
    GetItem(&'a ValueReference, &'a ValueReference),

    /// Sets the value at a given index
    SetItem(&'a ValueReference, &'a ValueReference, &'a ValueReference),

    /// Pushes a value onto a list
    Push(&'a ValueReference, &'a ValueReference),

    /// Removes a value from a list at a given index
    Remove(&'a ValueReference, &'a ValueReference),

    /// Adds two values together.
    Add(&'a ValueReference, &'a ValueReference),

    /// Subtracts two values.
    Sub(&'a ValueReference, &'a ValueReference),

    /// Multiplies two values.
    Mul(&'a ValueReference, &'a ValueReference),

    /// Divides two values.
    Div(&'a ValueReference, &'a ValueReference),

    /// Gets the remainder of two values.
    Mod(&'a ValueReference, &'a ValueReference),

    /// Raises a value to a power.
    Pow(&'a ValueReference, &'a ValueReference),

    /// Calls a function.
    Call(&'a ValueReference, Vec<&'a ValueReference>),

    /// Applies a function to each element of a list.
    Map(&'a ValueReference, &'a ValueReference),

    /// Applies a combining function to each element of a list, returning a single value.
    Reduce(&'a ValueReference, &'a ValueReference, &'a ValueReference),

    /// Gets the items in a list that match a given predicate.
    Filter(&'a ValueReference, &'a ValueReference),

    /// Initializes an object of the given custom type using the given data.
    Init(&'a ValueReference, Vec<&'a ValueReference>),
    //
    // /// Casts an object to a different type.
    // Cast(&'a ValueReference, &'a ValueReference),
}