pub(crate) mod sig;
pub(crate) mod val;
pub(crate) mod op_code;
pub(crate) mod op;
pub(crate) mod v2;

pub(crate) use sig::{FuncSig};
pub(crate) use val::{FuncVal};
pub(crate) use op_code::{OpCode};
pub(crate) use op::{FuncOp};