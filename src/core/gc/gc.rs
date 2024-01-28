use std::collections::HashMap;
use std::marker::PhantomData;
use crate::core::gc::{GarbageCollectable, GCObject, GCObjectType, GCPointer};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;

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

    /// Allocates a new value in the garbage collector and returns a pointer to it.
    pub fn allocate(&mut self, data: T) -> ExecResult<GCPointer<T>> where T: GarbageCollectable<T> {
        let mut obj = T::to_gc_object(data);
        let object_id = self.next_id;

        let mut ptr = GCPointer {
            id: object_id,
            counted: false,
            phantom: PhantomData,
        };

        // check if the obj has any child pointers
        let child_ptrs = obj.get_pointers();

        for child_ptr in child_ptrs {
            // if the pointer is uncounted, count it
            if !child_ptr.counted {
                self.count_ptr_by_id(child_ptr.id)?;
                child_ptr.counted = true;
            }
        }

        // count the pointer itself
        self.objects.insert(object_id, obj);
        self.next_id += 1;
        self.count_pointer(&mut ptr).map(|_| ptr)
    }

    /// Counts a pointer in the garbage collector.
    pub fn count_pointer(&mut self, ptr: &mut GCPointer<T>) -> ExecResult<StoredData> {
        // If the pointer is already counted, it does not need to be counted again.
        if ptr.counted {
            return Err("Pointer is already counted.".to_string());
        }

        // Get the object that the pointer points to.
        let obj_result = self.get_obj_mut(ptr.id);

        // If the object does not exist, return an error.
        let obj = match obj_result {
            Some(obj) => obj,
            None => return Err(format!("Pointer (id: {}) does not point to a valid object.", ptr.id)),
        };

        // increment the ref count of the object. And mark the pointer as counted.
        obj.ref_count += 1;
        ptr.counted = true;

        // Note: if the object has child pointers, we do not increment their ref counts.
        // The existence of the object will always only count as one reference.

        Ok(StoredData::NullStored)
    }

    fn count_ptr_by_id(&mut self, id: usize) -> ExecResult<StoredData> {
        let mut ptr = GCPointer {
            id,
            counted: false,
            phantom: PhantomData,
        };

        self.count_pointer(&mut ptr)
    }

    pub fn drop_pointer(&mut self, ptr: &mut GCPointer<T>) -> ExecResult<StoredData> {
        if !ptr.counted {
            return Err("Cannot drop a pointer that is not counted.".to_string());
        }
        ptr.counted = false;

        let mut to_remove: Vec<usize> = Vec::new();
        let mut to_process: Vec<usize> = Vec::new();
        to_process.push(ptr.id);

        while !to_process.is_empty() {
            let id = to_process.pop().unwrap();

            // Get the object that the pointer points to.
            let obj_result = self.get_obj_mut(id);
            let obj = match obj_result {
                Some(obj) => obj,
                None => return Err(format!("Pointer (id: {}) does not point to a valid object.", ptr.id)),
            };

            // Decrement the ref count of the object.
            obj.ref_count = obj.ref_count.saturating_sub(1);

            // If the ref count is 0, add it to the list of objects to remove.
            if obj.ref_count == 0 {
                to_remove.push(id);

                // if the obj is being removed, check if it has any child pointers.
                // We will need to decrement each child's ref count since their parent obj is being removed.
                let child_ptrs = obj.get_pointers();

                for child_ptr in child_ptrs {
                    if child_ptr.counted {
                        to_process.push(child_ptr.id);
                        child_ptr.counted = false;
                    }
                }
            }
        }

        to_remove.reverse();
        for id in to_remove {
            self.objects.remove(&id);
        }

        Ok(StoredData::NullStored)
    }

    /// Fills the value of a buffer.
    pub fn fill_buffer(&mut self, ptr: &GCPointer<T>, data: T) -> ExecResult<StoredData> where T: GarbageCollectable<T> {
        // If the pointer is not counted, it cannot be filled.
        if !ptr.counted {
            return Err("Cannot fill a pointer that is not counted.".to_string());
        }

        let obj = self.get_obj(ptr.id);

        if obj.is_none() {
            return Err("Buffer not found.".to_string());
        }

        if obj.unwrap().data_type != GCObjectType::Buffer {
            return Err("Buffer id is not a buffer.".to_string());
        }

        let ref_count = obj.unwrap().ref_count;

        let mut data_obj = T::to_gc_object(data);
        data_obj.ref_count = ref_count;

        // if the new object has any child pointers, count them
        let child_ptrs = data_obj.get_pointers();

        for child_ptr in child_ptrs {
            // if the pointer is uncounted, count it
            if !child_ptr.counted {
                self.count_ptr_by_id(child_ptr.id)?;
                child_ptr.counted = true;
            }
        }

        self.objects.insert(ptr.id, data_obj);

        Ok(StoredData::NullStored)
    }

    /// Gets the value that a pointer points to.
    pub fn get(&self, ptr: &GCPointer<T>) -> ExecResult<T> where T: GarbageCollectable<T> {
        // If the pointer is not counted, it cannot be dereferenced.
        // This implies that it has not been fully initialized.
        // TODO: add back this check. Removed for now because func op inputs are intentionally uncounted to avoid circular references.
        // It should still be possible to dereference them though.
        // if !ptr.counted {
        //     return Err("Cannot dereference a pointer that is not counted.".to_string());
        // }

        // Get the object that the pointer points to.
        let obj_result = self.get_obj(ptr.id);
        let obj: &GCObject<T> = match obj_result {
            Some(obj) => obj,
            None => return Err(format!("Pointer (id: {}) does not point to a valid object.", ptr.id)),
        };

        // Get the value of the object (automatically clones the value).
        let value = T::from_gc_object(obj);

        return value.map(|value| value);
    }

    fn get_obj(&self, id: usize) -> Option<&GCObject<T>> where T: GarbageCollectable<T> {
        self.objects.get(&id).map(|object| object)
    }

    fn get_obj_mut(&mut self, id: usize) -> Option<&mut GCObject<T>> where T: GarbageCollectable<T> {
        self.objects.get_mut(&id).map(|object| object)
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

impl<T> Clone for GCPointer<T> where T: GarbageCollectable<T> {
    /// Clones the pointer without incrementing the reference count.
    fn clone(&self) -> Self {
        GCPointer {
            id: self.id,
            phantom: PhantomData,
            counted: false,
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::core::vm::store_op::StoreOp;
    use crate::core::vm::VM;
    use crate::core::data::live::LiveData;

    #[test]
    fn test_gc() {
        let mut vm = VM::new(4);

        test_gc_helper1(&mut vm, "test");
        assert_eq!(vm.object_count(), 0);
    }

    fn test_gc_helper1(vm: &mut VM, value: &str) {
        vm.reset();

        let mut result = vm.execute_store(StoreOp::StoreString(value.to_string())).unwrap();

        let ref1 = result.get_mut(0).unwrap();
        let ref2 = ref1.clone();

        let val1 = vm.get_ref_value(ref1).unwrap();
        let val2 = vm.get_ref_value(&ref2).unwrap();

        assert_eq!(val1.as_live().as_string().unwrap(), Ok(value.to_string()));
        assert_eq!(val2.as_live().as_string().unwrap(), Ok(value.to_string()));

        assert_eq!(vm.object_count(), 1);

        assert_eq!(vm.ref_count(ref1).unwrap(), 2);

        drop(result);

        assert_eq!(vm.object_count(), 1);
        assert_eq!(vm.ref_count(&ref2).unwrap(), 1);

        drop(ref2);

        assert_eq!(vm.object_count(), 0);
    }
}