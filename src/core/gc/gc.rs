use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};
use crate::core::gc::{GarbageCollectable, GCObject, GCObjectData, GCObjectType, GCPointer};
use uuid::Uuid;
use crate::core::data::stored::StoredData;

/// A garbage collector that manages the lifetimes of objects.
#[derive(Debug)]
pub struct GarbageCollector<T> {
    pub objects: HashMap<usize, GCObject<T>>,
    next_id: usize,
}

impl<T> GarbageCollector<T> where T: GarbageCollectable<T> {
    pub fn new() -> Self {
        GarbageCollector {
            objects: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn increment_ref(&mut self, id: usize) {
        if let Some(object) = self.objects.get_mut(&id) {
            object.ref_count += 1;
        }
    }

    pub fn decrement_ref(&mut self, id: usize)  {
        let mut to_remove: Vec<usize> = Vec::new();

        let mut current_id = id;

        loop {
            let object: &mut GCObject<T> = self.get_obj_mut(current_id).unwrap();

            object.ref_count = object.ref_count.saturating_sub(1);

            if object.ref_count == 0 {
                println!("Removing object ({obj:?}) with id {id}", obj=object, id=id);
                to_remove.push(id);

                if object.data_type == GCObjectType::Pointer {
                    let pointer: &mut GCPointer<StoredData> = object.as_pointer().unwrap();

                    // set the pointer to no longer being counted
                    // this is because we are manually decrementing the ref count of the object it points to
                    // if we didn't do this, the pointer would try to decrement when dropped and cause a deadlock
                    pointer.counted = false;
                    current_id = pointer.id;
                }
                else {
                    break;
                }
            }
            else {
                break;
            }
        }

        // remove in backwards order to properly handle drop propagation
        to_remove.reverse();
        for id in to_remove {
            self.objects.remove(&id);
        }
    }

    pub fn allocate(&mut self, data: GCObject<T>) -> usize where T: GarbageCollectable<T> {
        let object = data;
        let object_id = self.next_id;
        self.objects.insert(object_id, object);
        self.next_id += 1;
        println!("Allocated object with id {}", object_id);
        object_id
    }

    pub fn get_obj(&self, id: usize) -> Option<&GCObject<T>> where T: GarbageCollectable<T> {
        self.objects.get(&id).map(|object| object)
    }

    fn get_obj_mut(&mut self, id: usize) -> Option<&mut GCObject<T>> where T: GarbageCollectable<T> {
        self.objects.get_mut(&id).map(|object| object)
    }

    pub fn get(&self, id: usize) -> Option<T> where T: GarbageCollectable<T> {
        self.objects.get(&id).and_then(|object| {
            let result = T::from_gc_object(object);
            return result
        })
    }

    // pub fn set(&mut self, id: usize, new_value: T) where T: GarbageCollectable<T> {
    //     let ref_count: usize = self.ref_count(id).unwrap_or(0);
    //     if let Some(object) = self.objects.get_mut(&id) {
    //         *object = new_value.to_gc_object(self);
    //         object.ref_count = ref_count;
    //     }
    // }

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

impl<T> GCPointer<T> where T: GarbageCollectable<T> {
    /// Allocates an object in the garbage collector and returns a pointer to it.
    pub fn new(value: T, gc: Arc<RwLock<GarbageCollector<T>>>) -> Self where T: GarbageCollectable<T> {
        let gc_object = value.to_gc_object();
        let id = gc.write().unwrap().allocate(gc_object);
        gc.write().unwrap().increment_ref(id);
        GCPointer { id, gc, phantom: PhantomData, counted: true }
    }
}

impl<T> Drop for GCPointer<T> where T: GarbageCollectable<T> {
    fn drop(&mut self) {
        println!("Dropping pointer: {id}", id=self.id);

        // If the pointer is not counted, then there is no need to decrement the reference count.
        if !self.counted {
            return;
        }
        self.gc.write().unwrap().decrement_ref(self.id);
    }
}

impl<T> Clone for GCPointer<T> where T: GarbageCollectable<T> {
    fn clone(&self) -> Self {
        let mut gc = match self.gc.try_write() {
            Ok(val) => val,
            Err(_) => panic!("Could not get write lock on garbage collector")
        };

        gc.increment_ref(self.id);
        GCPointer {
            id: self.id,
            gc: self.gc.clone(),
            phantom: PhantomData,
            counted: true,
        }
    }
}

impl<T> GCPointer<T> where T: GarbageCollectable<T> {
    /// Clones the pointer without incrementing the reference count.
    pub fn clone_unsafe(&self) -> Self {
        GCPointer {
            id: self.id,
            gc: self.gc.clone(),
            phantom: PhantomData,
            counted: false,
        }
    }
}