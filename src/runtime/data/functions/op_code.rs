use std::fmt;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::runtime::data::stored::StoredData;
use crate::runtime::vm::operator::ops::Operation;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpCode {
    #[serde(alias="typeof", alias="typeOf", alias="TypeOf", alias="TYPEOF", alias="TYPE_OF", alias="type_of", alias="Type_Of")]
    TypeOf,
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
    #[serde(alias="as_type", alias="astype", alias="asType", alias="AsType", alias="AS_TYPE", alias="ASTYPE", alias="AS_TYPE")]
    AsType,
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
    #[serde(alias="init", alias="Init", alias="INIT")]
    Init,
    #[serde(alias="static", alias="Static", alias="STATIC")]
    Static,
}

impl OpCode {
    /// Converts a list of arguments to an operation based on the opcode.
    pub fn to_operation(&self, args: &Vec<Arc<StoredData>>) -> Operation {
        match self {
            OpCode::TypeOf => Operation::TypeOf(args[0].clone()),
            OpCode::AsInt => Operation::AsInt(args[0].clone()),
            OpCode::AsFloat => Operation::AsFloat(args[0].clone()),
            OpCode::AsString => Operation::AsString(args[0].clone()),
            OpCode::AsBool => Operation::AsBool(args[0].clone()),
            OpCode::AsPointer => Operation::AsPointer(args[0].clone()),
            OpCode::AsList => Operation::AsList(args[0].clone()),
            OpCode::AsDictionary => Operation::AsDictionary(args[0].clone()),
            OpCode::AsType => Operation::AsType(args[0].clone()),
            OpCode::If => Operation::If(args[0].clone(), args[1].clone(), args[2].clone()),
            OpCode::Not => Operation::Not(args[0].clone()),
            OpCode::And => Operation::And(args[0].clone(), args[1].clone()),
            OpCode::Or => Operation::Or(args[0].clone(), args[1].clone()),
            OpCode::Equal => Operation::Equal(args[0].clone(), args[1].clone()),
            OpCode::LessThan => Operation::LessThan(args[0].clone(), args[1].clone()),
            OpCode::GreaterThan => Operation::GreaterThan(args[0].clone(), args[1].clone()),
            OpCode::IsNull => Operation::IsNull(args[0].clone()),
            OpCode::Length => Operation::Length(args[0].clone()),
            OpCode::Get => Operation::GetItem(args[0].clone(), args[1].clone()),
            OpCode::Set => Operation::SetItem(args[0].clone(), args[1].clone(), args[2].clone()),
            OpCode::Push => Operation::Push(args[0].clone(), args[1].clone()),
            OpCode::Remove => Operation::Remove(args[0].clone(), args[1].clone()),
            OpCode::Add => Operation::Add(args[0].clone(), args[1].clone()),
            OpCode::Sub => Operation::Sub(args[0].clone(), args[1].clone()),
            OpCode::Mul => Operation::Mul(args[0].clone(), args[1].clone()),
            OpCode::Div => Operation::Div(args[0].clone(), args[1].clone()),
            OpCode::Mod => Operation::Mod(args[0].clone(), args[1].clone()),
            OpCode::Pow => Operation::Pow(args[0].clone(), args[1].clone()),
            OpCode::Call => Operation::Call(args[0].clone(), args[1..].to_vec()),
            OpCode::Map => Operation::Map(args[0].clone(), args[1].clone()),
            OpCode::Reduce => Operation::Reduce(args[0].clone(), args[1].clone(), args[2].clone()),
            OpCode::Filter => Operation::Filter(args[0].clone(), args[1].clone()),
            OpCode::Init => Operation::Init(args[0].clone(), args[1..].to_vec()),
            OpCode::Static => panic!("Cannot convert OpCode::Static to Operation"),
        }
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpCode::TypeOf => write!(f, "type_of"),
            OpCode::AsInt => write!(f, "as_int"),
            OpCode::AsFloat => write!(f, "as_float"),
            OpCode::AsString => write!(f, "as_string"),
            OpCode::AsBool => write!(f, "as_bool"),
            OpCode::AsPointer => write!(f, "as_pointer"),
            OpCode::AsList => write!(f, "as_list"),
            OpCode::AsDictionary => write!(f, "as_dictionary"),
            OpCode::AsType => write!(f, "as_type"),
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
            OpCode::Init => write!(f, "init"),
            OpCode::Static => write!(f, "static"),
        }
    }
}