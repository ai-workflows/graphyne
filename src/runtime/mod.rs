mod fundamentals;
pub(crate) mod data;
pub(crate) mod gc;
pub(crate) mod vm;
pub(crate) mod mmu;


pub(crate) use fundamentals::{ExecResult, Symbol, SymbolPath};
pub(crate) use data::live::types::Type;
