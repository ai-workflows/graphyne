pub(crate) mod live_data;
pub(crate) mod num;
pub(crate) mod string;
pub(crate) mod list;
pub(crate) mod pointer;
pub(crate) mod dict;
pub(crate) mod bool;
pub(crate) mod func;
mod null;
pub mod types;
mod object;
mod r#static;


pub(crate) use live_data::{BoolLive, DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, LiveData, NullLive, PointerLive, StringLive, TypeLive, ObjectLive, StaticRefLive};
