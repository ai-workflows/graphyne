use std::fmt;
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpCode {
    AsInt,
    AsFloat,
    AsString,
    AsBool,
    AsPointer,
    AsList,
    AsDictionary,
    If,
    Not,
    And,
    Or,
    Equal,
    LessThan,
    GreaterThan,
    Length,
    GetItem,
    SetItem,
    Push,
    Remove,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Call,
    Map,
    Reduce,
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
            OpCode::Length => Operation::Length(args[0]),
            OpCode::GetItem => Operation::GetItem(args[0], args[1]),
            OpCode::SetItem => Operation::SetItem(args[0], args[1], args[2]),
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
            OpCode::Length => write!(f, "length"),
            OpCode::GetItem => write!(f, "get_item"),
            OpCode::SetItem => write!(f, "set_item"),
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