mod fundamentals;
pub(crate) mod data;
pub(crate) mod vm;
pub(crate)mod static_state;


pub(crate) use fundamentals::{ExecResult, Symbol, SymbolPath};
pub(crate) use data::live::types::Type;
