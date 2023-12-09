use std::sync::{Arc, Mutex};
use crate::core::gc::{GarbageCollector, GCObject};

pub trait GarbageCollectable<T>: Sized {
    fn from_gc_object(object: &GCObject<T>) -> Option<Self>;
    fn to_gc_object(&self) -> GCObject<T>;
}