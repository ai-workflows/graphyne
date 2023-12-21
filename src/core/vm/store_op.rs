use std::collections::HashMap;
use crate::api::functions::FunctionGraph;
use crate::core::data::functions::{FuncOp, FuncSig, FuncVal, OpCode};
use crate::core::data::live::{BoolLive, FloatLive, IntLive, StringLive};
use crate::core::data::stored::StoredData;
use crate::core::Symbol;
use crate::core::vm::value_ref::ValueReference;

pub enum StoreOp<'a>  {
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
    StoreDict(HashMap<Symbol, &'a ValueReference<'a>>),

    /// Stores a literal function in memory.
    /// input_vals: Reference to the func value nodes that args will be binded to when the function is called.
    /// output_vals: Reference to the func value nodes that the function will return when it is called.
    /// constants: Reference to the func value nodes that are constants used by the function.
    StoreFunction(Vec<&'a ValueReference<'a>>, Vec<&'a ValueReference<'a>>, Vec<&'a ValueReference<'a>>),

    /// Stores a literal function value in memory.
    /// dependents: list of refs to the func op nodes that depend on this func val.
    /// constant: an optional ref to a constant value that this func val is initialized to.
    /// is_self: whether this func val is a pointer to the function's class context.
    StoreFunctionVal(Vec<&'a ValueReference<'a>>, Option<&'a ValueReference<'a>>, bool),

    /// Stores a literal function operation in memory.
    /// opcode: The opcode of the operation.
    /// input_vals: Reference to the func value nodes that are used as inputs for this operation.
    /// output_vals: Reference to the func value nodes that are the outputs of this operation.
    StoreFunctionOp(OpCode, Vec<&'a ValueReference<'a>>, Vec<&'a ValueReference<'a>>),

    /// Stores a function graph in memory.
    /// The second argument is a reference to the class (as a dict) that the func belongs to (if any).
    StoreFunctionGraph(FunctionGraph, Option<&'a ValueReference<'a>>),

    /// Creates a buffer in memory (a pointer to nothing).
    CreateBuffer,
}

impl<'a> StoreOp<'a> {
    /// Returns the stored data from the args for store operations.
    pub fn get_stored_data(self) -> Option<StoredData> {
        match self {
            StoreOp::StoreInt(data) => Some(StoredData::IntStored(data)),
            StoreOp::StoreFloat(data) => Some(StoredData::FloatStored(data)),
            StoreOp::StoreString(data) => Some(StoredData::StringStored(data)),
            StoreOp::StoreBool(data) => Some(StoredData::BoolStored(data)),
            StoreOp::StorePointer(data) => Some(StoredData::PointerStored(data.pointer.clone())),
            StoreOp::StoreList(value) => Some(StoredData::ListStored(value.iter().map(|v| v.pointer.clone()).collect())),
            StoreOp::StoreDict(value) => Some(StoredData::DictStored(value.iter().map(|(k, v)| (k.clone(), v.pointer.clone())).collect())),
            StoreOp::StoreFunction(input_vals, output_vals, constants) => Some(StoredData::FuncStored(FuncSig{
                input_vals: input_vals.iter().map(|v| v.pointer.clone()).collect(),
                output_vals: output_vals.iter().map(|v| v.pointer.clone()).collect(),
                constant_vals: constants.iter().map(|v| v.pointer.clone()).collect(),
            })),
            StoreOp::StoreFunctionVal(dependents, constant, is_self) => Some(StoredData::FuncValStored(FuncVal{
                guid: uuid::Uuid::new_v4().to_string(),
                dependents: dependents.iter().map(|v| v.pointer.clone()).collect(),
                constant: constant.map(|v| v.pointer.clone()),
                is_self,
            })),
            StoreOp::StoreFunctionOp(op_code, input_vals, output_val) => Some(StoredData::FuncOpStored(FuncOp{
                guid: uuid::Uuid::new_v4().to_string(),
                opcode: op_code,
                input_vals: input_vals.iter().map(|v| v.pointer.clone()).collect(),
                output_vals: output_val.iter().map(|v| v.pointer.clone()).collect(),
            })),
            _ => None,
        }
    }
}