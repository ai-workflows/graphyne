use crate::core::gc::GCObject;

pub trait GarbageCollectable: Sized {
    fn from_gc_object(object: &GCObject) -> Option<Self>;
    fn to_gc_object(&self) -> GCObject;
}