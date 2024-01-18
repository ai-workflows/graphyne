use std::fmt;
use serde::{Deserialize, Serialize};
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpCode {
    #[serde(alias="as_int", alias="asint", alias="asInt", alias="AsInt", alias="AS_INT", alias="ASINT", alias="AS_INT")]
    AsInt,
    #[serde(alias="as_float", alias="asfloat", alias="asFloat", alias="AsFloat", alias="AS_FLOAT", alias="ASFLOAT", alias="AS_FLOAT")]
    AsFloat,
    #[serde(alias="as_string", alias="asstring", alias="asString", alias="AsString", alias="AS_STRING", alias="ASSTRING", alias="AS_STRING")]
    AsString,
    #[serde(alias="as_bool", alias="asbool", alias="asBool", alias="AsBool", alias="AS_BOOL", alias="ASBOOL", alias="AS_BOOL")]
    AsBool,
    #[serde(alias="as_pointer", alias="aspointer", alias="asPointer", alias="AsPointer", alias="AS_POINTER", alias="ASPOINTER", alias="AS_POINTER")]
    AsPointer,
    #[serde(alias="as_list", alias="aslist", alias="asList", alias="AsList", alias="AS_LIST", alias="ASLIST", alias="AS_LIST")]
    AsList,
    #[serde(alias="as_dictionary", alias="asdictionary", alias="asDictionary", alias="AsDictionary", alias="AS_DICTIONARY", alias="ASDICTIONARY", alias="AS_DICTIONARY")]
    AsDictionary,
    #[serde(alias="if", alias="If", alias="IF")]
    If,
    #[serde(alias="not", alias="Not", alias="NOT")]
    Not,
    #[serde(alias="and", alias="And", alias="AND")]
    And,
    #[serde(alias="or", alias="Or", alias="OR")]
    Or,
    #[serde(alias="equal", alias="Equal", alias="EQUAL")]
    Equal,
    #[serde(alias="less_than", alias="lessThan", alias="LessThan", alias="LESSTHAN", alias="LESS_THAN")]
    LessThan,
    #[serde(alias="greater_than", alias="greaterThan", alias="GreaterThan", alias="GREATERTHAN", alias="GREATER_THAN")]
    GreaterThan,
    #[serde(alias="is_null", alias="isNull", alias="IsNull", alias="ISNULL", alias="IS_NULL")]
    IsNull,
    #[serde(alias="length", alias="Length", alias="LENGTH")]
    Length,
    #[serde(alias="get", alias="Get", alias="GET")]
    Get,
    #[serde(alias="set", alias="Set", alias="SET")]
    Set,
    #[serde(alias="push", alias="Push", alias="PUSH")]
    Push,
    #[serde(alias="remove", alias="Remove", alias="REMOVE")]
    Remove,
    #[serde(alias="add", alias="Add", alias="ADD")]
    Add,
    #[serde(alias="sub", alias="Sub", alias="SUB")]
    Sub,
    #[serde(alias="mul", alias="Mul", alias="MUL")]
    Mul,
    #[serde(alias="div", alias="Div", alias="DIV")]
    Div,
    #[serde(alias="mod", alias="Mod", alias="MOD")]
    Mod,
    #[serde(alias="pow", alias="Pow", alias="POW")]
    Pow,
    #[serde(alias="call", alias="Call", alias="CALL")]
    Call,
    #[serde(alias="map", alias="Map", alias="MAP")]
    Map,
    #[serde(alias="reduce", alias="Reduce", alias="REDUCE")]
    Reduce,
    #[serde(alias="filter", alias="Filter", alias="FILTER")]
    Filter,
}

impl OpCode {
    /// Converts a list of arguments to an operation based on the opcode.
    pub fn to_operation<'a>(&self, args: &Vec<&'a ValueReference<'a>>) -> Operation<'a> {
        match self {
            OpCode::AsInt => Operation::AsInt(args[0]),
            OpCode::AsFloat => Operation::AsFloat(args[0]),
            OpCode::AsString => Operation::AsString(args[0]),
            OpCode::AsBool => Operation::AsBool(args[0]),
            OpCode::AsPointer => Operation::AsPointer(args[0]),
            OpCode::AsList => Operation::AsList(args[0]),
            OpCode::AsDictionary => Operation::AsDictionary(args[0]),
            OpCode::If => Operation::If(args[0], args[1], args[2]),
            OpCode::Not => Operation::Not(args[0]),
            OpCode::And => Operation::And(args[0], args[1]),
            OpCode::Or => Operation::Or(args[0], args[1]),
            OpCode::Equal => Operation::Equal(args[0], args[1]),
            OpCode::LessThan => Operation::LessThan(args[0], args[1]),
            OpCode::GreaterThan => Operation::GreaterThan(args[0], args[1]),
            OpCode::IsNull => Operation::IsNull(args[0]),
            OpCode::Length => Operation::Length(args[0]),
            OpCode::Get => Operation::GetItem(args[0], args[1]),
            OpCode::Set => Operation::SetItem(args[0], args[1], args[2]),
            OpCode::Push => Operation::Push(args[0], args[1]),
            OpCode::Remove => Operation::Remove(args[0], args[1]),
            OpCode::Add => Operation::Add(args[0], args[1]),
            OpCode::Sub => Operation::Sub(args[0], args[1]),
            OpCode::Mul => Operation::Mul(args[0], args[1]),
            OpCode::Div => Operation::Div(args[0], args[1]),
            OpCode::Mod => Operation::Mod(args[0], args[1]),
            OpCode::Pow => Operation::Pow(args[0], args[1]),
            OpCode::Call => Operation::Call(args[0], args[1..].to_vec()),
            OpCode::Map => Operation::Map(args[0], args[1]),
            OpCode::Reduce => Operation::Reduce(args[0], args[1], args[2]),
            OpCode::Filter => Operation::Filter(args[0], args[1]),
        }
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpCode::AsInt => write!(f, "as_int"),
            OpCode::AsFloat => write!(f, "as_float"),
            OpCode::AsString => write!(f, "as_string"),
            OpCode::AsBool => write!(f, "as_bool"),
            OpCode::AsPointer => write!(f, "as_pointer"),
            OpCode::AsList => write!(f, "as_list"),
            OpCode::AsDictionary => write!(f, "as_dictionary"),
            OpCode::If => write!(f, "if"),
            OpCode::Not => write!(f, "not"),
            OpCode::And => write!(f, "and"),
            OpCode::Or => write!(f, "or"),
            OpCode::Equal => write!(f, "equal"),
            OpCode::LessThan => write!(f, "less_than"),
            OpCode::GreaterThan => write!(f, "greater_than"),
            OpCode::IsNull => write!(f, "is_null"),
            OpCode::Length => write!(f, "length"),
            OpCode::Get => write!(f, "get_item"),
            OpCode::Set => write!(f, "set_item"),
            OpCode::Push => write!(f, "push"),
            OpCode::Remove => write!(f, "remove"),
            OpCode::Add => write!(f, "add"),
            OpCode::Sub => write!(f, "sub"),
            OpCode::Mul => write!(f, "mul"),
            OpCode::Div => write!(f, "div"),
            OpCode::Mod => write!(f, "mod"),
            OpCode::Pow => write!(f, "pow"),
            OpCode::Call => write!(f, "call"),
            OpCode::Map => write!(f, "map"),
            OpCode::Reduce => write!(f, "reduce"),
            OpCode::Filter => write!(f, "filter"),
        }
    }
}