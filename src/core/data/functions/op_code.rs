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
        }
    }
}