use std::sync::Arc;
use crate::runtime::ExecResult;
use crate::runtime::gc::{GCObject, GCPointer};

pub trait GarbageCollectable<T>: Sized {
    fn clone_from_gc_object(object: &GCObject<T>) -> ExecResult<Self>;
    fn to_gc_object(self) -> GCObject<Arc<T>>;
    // fn get_pointers(&mut self) -> Vec<&mut PointerLive>;

    fn from_gc_object(object: &GCObject<T>) -> ExecResult<&Self>;
    fn get_pointers(&self) -> Vec<&GCPointer<T>> where T: GarbageCollectable<T>;
    fn get_pointers_mut(&mut self) -> Vec<&mut GCPointer<T>> where T: GarbageCollectable<T>;
}