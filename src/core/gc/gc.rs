use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use crate::core::gc::{GarbageCollectable, GCObject, GCPointer};

/// A garbage collector that manages the lifetimes of objects.
#[derive(Debug)]
pub struct GarbageCollector {
    pub objects: HashMap<usize, GCObject>,
    next_id: usize,
}

impl GarbageCollector {
    pub fn new() -> Self {
        GarbageCollector {
            objects: HashMap::new(),
            next_id: 0,
        }
    }

    fn increment_ref(&mut self, id: usize) {
        if let Some(object) = self.objects.get_mut(&id) {
            object.ref_count += 1;
        }
    }

    fn decrement_ref(&mut self, id: usize) {
        if let Some(object) = self.objects.get_mut(&id) {
            object.ref_count = object.ref_count.saturating_sub(1);
            if object.ref_count == 0 {
                self.objects.remove(&id);
            }
        }
    }

    pub fn allocate<T>(&mut self, data: T) -> usize where T: GarbageCollectable {
        let object = data.to_gc_object();
        let object_id = self.next_id;
        self.objects.insert(object_id, object);
        self.next_id += 1;
        object_id
    }

    pub fn get<T>(&self, id: usize) -> Option<T> where T: GarbageCollectable {
        self.objects.get(&id).and_then(|object| T::from_gc_object(object))
    }

    pub fn set<T>(&mut self, id: usize, new_value: T) where T: GarbageCollectable {
        let ref_count: usize = self.ref_count(id).unwrap_or(0);
        if let Some(object) = self.objects.get_mut(&id) {
            *object = new_value.to_gc_object();
            object.ref_count = ref_count;
        }
    }

    pub fn ref_count(&self, id: usize) -> Option<usize> {
        self.objects.get(&id).map(|object| object.ref_count)
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.next_id = 0;
    }

    /// returns the number of objects in the garbage collector
    pub fn len(&self) -> usize {
        self.objects.len()
    }
}

impl<T> GCPointer<T> where T: GarbageCollectable {
    /// Allocates an object in the garbage collector and returns a pointer to it.
    pub fn new(value: T, gc: Arc<Mutex<GarbageCollector>>) -> Self where T: GarbageCollectable {
        let id = gc.lock().unwrap().allocate(value);
        gc.lock().unwrap().increment_ref(id);
        GCPointer { id, gc, phantom: PhantomData }
    }
}

impl<T> Drop for GCPointer<T> where T: GarbageCollectable {
    fn drop(&mut self) {
        self.gc.lock().unwrap().decrement_ref(self.id);
    }
}

impl<T> Clone for GCPointer<T> where T: GarbageCollectable {
    fn clone(&self) -> Self {
        self.gc.lock().unwrap().increment_ref(self.id);
        GCPointer {
            id: self.id,
            gc: self.gc.clone(),
            phantom: PhantomData,
        }
    }
}