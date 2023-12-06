mod gc;
mod pointer;
mod obj;
mod collectable;

pub(crate) use gc::GarbageCollector;
pub(crate) use pointer::GCPointer;
pub(crate) use obj::{GCObject, GCObjectType};
pub(crate) use collectable::GarbageCollectable;