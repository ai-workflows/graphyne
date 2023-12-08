use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use crate::core::gc::{GarbageCollectable, GarbageCollector};

/// Represents a pointer to a value that is being managed by the garbage collector.
#[derive(Debug)]
pub struct GCPointer<T> where T: GarbageCollectable<T> {
    pub id: usize,
    pub gc: Arc<Mutex<GarbageCollector<T>>>,
    pub phantom: PhantomData<T>
}

impl<T> GCPointer<T> where T: GarbageCollectable<T> {
    pub fn get(&self) -> Option<T> {
        self.gc.lock().unwrap().get(self.id)
    }

    pub fn set(&self, new_value: T)  {
        self.gc.lock().unwrap().set(self.id, new_value);
    }

    pub fn ref_count(&self) -> Option<usize> {
        self.gc.lock().unwrap().ref_count(self.id)
    }

    pub fn increment_ref(&self) {
        self.gc.lock().unwrap().increment_ref(self.id);
    }

    pub fn decrement_ref(&self) {
        self.gc.lock().unwrap().decrement_ref(self.id);
    }
}

impl<T> PartialEq for GCPointer<T> where T: GarbageCollectable<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}