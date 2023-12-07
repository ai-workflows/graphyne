use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use crate::core::gc::{GarbageCollectable, GarbageCollector};

/// Represents a pointer to a value that is being managed by the garbage collector.
#[derive(Debug)]
pub struct GCPointer<T> where T: GarbageCollectable {
    pub id: usize,
    pub gc: Arc<Mutex<GarbageCollector>>,
    pub phantom: PhantomData<T>,
}

impl<T> GCPointer<T> where T: GarbageCollectable {
    pub fn get(&self) -> Option<T> {
        self.gc.lock().unwrap().get(self.id)
    }

    pub fn set(&self, new_value: T)  {
        self.gc.lock().unwrap().set(self.id, new_value);
    }

    pub fn ref_count(&self) -> Option<usize> {
        self.gc.lock().unwrap().ref_count(self.id)
    }
}