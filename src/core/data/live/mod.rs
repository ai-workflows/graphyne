pub(crate) mod live_data;
pub(crate) mod num;
pub(crate) mod string;
pub(crate) mod list;
pub(crate) mod pointer;
pub(crate) mod dict;
pub(crate) mod bool;
pub(crate) mod func;


pub(crate) use live_data::{LiveData, IntLive, FloatLive, StringLive, PointerLive, ListLive, DictLive, BoolLive, FuncLive, FuncOpLive, FuncValLive};
