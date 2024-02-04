use crate::core::data::live::{PointerLive, TypeLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::GCPointer;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    /// Gets a copy of the stored data referenced by the given value reference.
    pub fn get_ref_value(&self, arg: &ValueReference) -> ExecResult<StoredData> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = match self.state.try_read() {
            Ok(value) => value,
            Err(_) => return Err("Could not get read lock on VM state".to_string()),
        };

        let get_result = gc.get(&arg.pointer);

        get_result.map(|value| value)
    }

    pub fn ref_count(&self, arg: &ValueReference) -> ExecResult<usize> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = match self.state.try_read() {
            Ok(value) => value,
            Err(_) => return Err("Could not get read lock on VM state".to_string()),
        };

        match gc.ref_count(arg.pointer.id) {
            Some(count) => Ok(count),
            None => Err("Could not get reference count".to_string()),
        }
    }

    /// Converts a pointer into a value reference that can be used by the VM's caller.
    /// Counts the pointer if it is uncounted.
    pub fn value_ref_from_ptr(&self, mut ptr: GCPointer<StoredData>) -> ExecResult<ValueReference> {
        // if the pointer is uncounted, we need to manually count it
        if !ptr.counted {
            let mut gc = match self.state.try_write() {
                Ok(value) => value,
                Err(_) => return Err("Could not get write lock on VM state".to_string()),
            };

            match gc.count_pointer(&mut ptr) {
                Ok(_) => {},
                Err(e) => return Err(format!("Could not count pointer: {}", e)),
            }
        }

        let result = ValueReference::new(ptr, &self);

        return Ok(result)
    }

    pub fn get_ptr_value(&self, ptr: &GCPointer<StoredData>) -> ExecResult<StoredData> {
        let gc = match self.state.try_read() {
            Ok(value) => value,
            Err(_) => return Err("Could not get read lock on VM state".to_string()),
        };

        let get_result = gc.get(ptr);

        get_result.map(|value| value)
    }

    /// Stores a value in the VM's state, returning a reference to the stored value
    pub fn store_value(&self, value: StoredData) -> ExecResult<Vec<ValueReference>> {
        // try to get write lock
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        // allocate value
        let ptr = match gc.allocate(value) {
            Ok(ptr) => ptr,
            Err(msg) => return Err(format!("Could not allocate value: {}", msg).to_string()),
        };

        // create value reference
        return match self.value_ref_from_ptr(ptr) {
            Ok(value_ref) => Ok(vec![value_ref]),
            Err(msg) => Err(format!("Could not create value reference: {}", msg).to_string()),
        }
    }

    /// Drops a reference to a value from the VM's state, decrementing the reference count.
    pub fn drop_reference(&self, reference: &mut ValueReference) {
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => panic!("Could not get write lock on VM state"),
        };

        let drop_result = gc.drop_pointer(&mut reference.pointer);

        if let Err(msg) = drop_result {
            panic!("Could not drop pointer: {}", msg);
        }
    }

    /// Clones a value reference, incrementing the reference count.
    pub fn clone_reference(&self, reference: &ValueReference) -> ExecResult<ValueReference> {
        let mut cloned_ptr = reference.pointer.clone();

        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        match gc.count_pointer(&mut cloned_ptr) {
            Ok(_) => {}
            Err(msg) => return Err(msg),
        }

        let new_reference = ValueReference::new(cloned_ptr, self);

        return Ok(new_reference)
    }

    /// Fills a buffer with the given value
    pub fn execute_fill_buffer(&self, buffer: &ValueReference, value: StoredData) -> ExecResult<Vec<ValueReference>> {
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        let fill_result = gc.fill_buffer(&buffer.pointer, value);

        if let Err(msg) = fill_result {
            return Err(msg);
        }

        return Ok(vec![])
    }



    /// Gets the live type of stored data.
    pub fn get_stored_type(&self, arg: &StoredData) -> ExecResult<TypeLive> {
        let type_ptr = match self.get_stored_type_ptr(arg) {
            Ok(ptr) => ptr,
            Err(msg) => return Err(msg),
        };

        let type_ref = match self.value_ref_from_ptr(type_ptr) {
            Ok(type_ref) => type_ref,
            Err(msg) => return Err(msg),
        };

        let type_value = match self.get_ref_value(&type_ref) {
            Ok(type_value) => type_value,
            Err(msg) => return Err(msg),
        };

        return match type_value {
            StoredData::TypeStored(type_live) => Ok(type_live),
            _ => Err("Could not get type of argument, type is a non-type value".to_string()),
        };
    }

    /// Converts a value ref to a pointer (doesn't have a reference to the vm).
    /// Counts the new pointer so the ref count stays the same.
    pub fn counted_ptr_from_value_ref(&self, mut value_ref: ValueReference) -> ExecResult<PointerLive> {
        let mut gc = match self.state.try_write() {
            Ok(value) => value,
            Err(_) => return Err("Could not get write lock on VM state".to_string()),
        };

        // clone the pointer
        let mut ptr = value_ref.pointer.clone();

        // count the pointer
        match gc.count_pointer(&mut ptr) {
            Ok(_) => {},
            Err(_) => return Err("Could not count pointer".to_string()),
        }

        // drop the value reference
        drop(value_ref);

        Ok(ptr)
    }
}