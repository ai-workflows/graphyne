use std::collections::HashMap;
use crate::core::data::functions::{FuncOp, FuncSig, FuncVal, OpCode};
use crate::core::data::live::{BoolLive, FloatLive, IntLive, StringLive};
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;


/// Represents an operation that can be performed on data.
/// Each operation contains a pointer to the data for its operands.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Operation<'a> {
    /// Stores a literal int in memory.
    StoreInt(IntLive),

    /// Stores a literal float in memory.
    StoreFloat(FloatLive),

    /// Stores a literal string in memory.
    StoreString(StringLive),

    /// Stores a literal boolean in memory.
    StoreBool(BoolLive),

    /// Stores a literal pointer in memory.
    StorePointer(&'a ValueReference<'a>),

    /// Stores a literal list in memory.
    StoreList(Vec<&'a ValueReference<'a>>),

    /// Stores a literal dictionary in memory.
    StoreDict(HashMap<StringLive, &'a ValueReference<'a>>),

    /// Stores a literal function in memory.
    /// input_vals: Reference to the func value nodes that args will be binded to when the function is called.
    /// output_vals: Reference to the func value nodes that the function will return when it is called.
    StoreFunction(Vec<&'a ValueReference<'a>>, Vec<&'a ValueReference<'a>>),

    /// Stores a literal function value in memory.
    /// dependents: list of refs to the func op nodes that depend on this func val.
    StoreFunctionVal(Vec<&'a ValueReference<'a>>),

    /// Stores a literal function operation in memory.
    /// opcode: The opcode of the operation.
    /// input_vals: Reference to the func value nodes that are used as inputs for this operation.
    /// output_val: Reference to the func value node that is the output of this operation.
    StoreFunctionOp(OpCode, Vec<&'a ValueReference<'a>>, &'a ValueReference<'a>),
    
    /// Creates a buffer in memory (a pointer to nothing).
    CreateBuffer,
    
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
}

impl<'a> Operation<'a> {
    /// Returns the stored data from the args for store operations.
    pub fn get_stored_data(self) -> Option<StoredData> {
        match self {
            Operation::StoreInt(data) => Some(StoredData::IntStored(data)),
            Operation::StoreFloat(data) => Some(StoredData::FloatStored(data)),
            Operation::StoreString(data) => Some(StoredData::StringStored(data)),
            Operation::StoreBool(data) => Some(StoredData::BoolStored(data)),
            Operation::StorePointer(data) => Some(StoredData::PointerStored(data.pointer.clone())),
            Operation::StoreList(value) => Some(StoredData::ListStored(value.iter().map(|v| v.pointer.clone()).collect())),
            Operation::StoreDict(value) => Some(StoredData::DictStored(value.iter().map(|(k, v)| (k.clone(), v.pointer.clone())).collect())),
            Operation::StoreFunction(input_vals, output_vals) => Some(StoredData::FuncStored(FuncSig{input_vals: input_vals.iter().map(|v| v.pointer.clone()).collect(), output_vals: output_vals.iter().map(|v| v.pointer.clone()).collect()})),
            Operation::StoreFunctionVal(value) => Some(StoredData::FuncValStored(FuncVal{guid: uuid::Uuid::new_v4().to_string(), dependents: value.iter().map(|v| v.pointer.clone()).collect()})),
            Operation::StoreFunctionOp(op_code, input_vals, output_val) => Some(StoredData::FuncOpStored(FuncOp{opcode: op_code, input_vals: input_vals.iter().map(|v| v.pointer.clone()).collect(), output_val: output_val.pointer.clone()})),

            _ => None,
        }
    }
}