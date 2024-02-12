use crate::runtime::ExecResult;
use crate::runtime::gc::{GCObject};

pub trait GarbageCollectable<T>: Sized {
    fn from_gc_object(object: &GCObject<T>) -> ExecResult<Self>;
    fn to_gc_object(self) -> GCObject<T>;
    // fn get_pointers(&mut self) -> Vec<&mut PointerLive>;
}