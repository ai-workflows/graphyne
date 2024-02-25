mod gc;
mod pointer;
mod obj;
mod collectable;

pub(crate) use gc::GarbageCollector;
pub(crate) use pointer::GCPointer;
pub(crate) use obj::{GCObject};
pub(crate) use collectable::GarbageCollectable;
// pub(crate) use global_map::{register_gc, get_gc};