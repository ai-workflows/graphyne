use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::runtime::data::live::{PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::gc::{GarbageCollector, GCPointer};
use crate::runtime::mmu::functions::store_function;
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::ops::results::get_stored_type_ptr;

macro_rules! allocate_primitive_type {
    ($state:ident, $prims:ident, $type:path) => {
        match $state.allocate(StoredData::TypeStored($type)) {
            Ok(ptr) => $prims.insert($type, ptr),
            Err(msg) => return Err(msg),
        };
    };
}

pub struct MMU {
    pub state: Arc<RwLock<GarbageCollector<StoredData>>>,
    pub primitive_types: HashMap<TypeLive, PointerLive>,
}

impl MMU {
    pub fn new() -> Self {
        let state = GarbageCollector::new();
        let mut res = MMU {
            state: Arc::new(RwLock::new(state)),
            primitive_types: HashMap::new(),
        };

        // load in the primitive types
        match res.allocate_primitive_types() {
            Ok(types) => types,
            Err(msg) => panic!("Could not allocate primitive types: {}", msg),
        };

        res
    }

    fn allocate_primitive_types(&mut self) -> ExecResult<()> {
        let mut prims: HashMap<TypeLive, PointerLive> = HashMap::new();
        let mut state = self.state.write().unwrap();

        allocate_primitive_type!(state, prims, TypeLive::Integer);
        allocate_primitive_type!(state, prims, TypeLive::Float);
        allocate_primitive_type!(state, prims, TypeLive::String);
        allocate_primitive_type!(state, prims, TypeLive::Boolean);
        allocate_primitive_type!(state, prims, TypeLive::Pointer);
        allocate_primitive_type!(state, prims, TypeLive::List);
        allocate_primitive_type!(state, prims, TypeLive::Dictionary);
        allocate_primitive_type!(state, prims, TypeLive::Function);
        allocate_primitive_type!(state, prims, TypeLive::FunctionVal);
        allocate_primitive_type!(state, prims, TypeLive::FunctionOp);
        allocate_primitive_type!(state, prims, TypeLive::Null);
        allocate_primitive_type!(state, prims, TypeLive::Type);
        allocate_primitive_type!(state, prims, TypeLive::Dynamic);

        self.primitive_types = prims;

        Ok(())
    }

    /// Reset the VM state, clearing all stored data
    pub fn reset(&mut self) {
        self.state.write().unwrap().clear();

        // load in the primitive types
        match self.allocate_primitive_types() {
            Ok(types) => types,
            Err(msg) => panic!("Could not allocate primitive types: {}", msg),
        };
    }

    /// Returns the number of objects currently stored in the VM excluding primitive types
    pub fn object_count(&self) -> usize {
        self.state.read().unwrap().len() - self.primitive_types.len()
    }

    /// Returns the number of objects currently stored in the VM including primitive types
    pub fn object_count_full(&self) -> usize {
        self.state.read().unwrap().len()
    }

    /// Gets the object count of a newly initialized VM (only contains primitive types)
    pub fn initial_count(&self) -> usize {
        self.primitive_types.len()
    }

    /// Gets a reef of the stored data referenced by the given value reference.
    pub fn get_ref_value(&self, arg: &ValueReference) -> ExecResult<Arc<StoredData>> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = self.state.read()
            .unwrap_or_else(|e| panic!("Could not get read lock on VM state, the lock is poisoned: {}", e));

        let get_result = gc.get(&arg.pointer)?;
        return Ok(get_result);
    }

    pub fn ref_count(&self, arg: &ValueReference) -> ExecResult<usize> {
        if !arg.is_alive() {
            return Err("Cannot use dead reference as an argument".to_string());
        }

        let gc = self.state.read()
            .unwrap_or_else(|e| panic!("Could not get read lock on VM state, the lock is poisoned: {}", e));

        match gc.ref_count(arg.pointer.id) {
            Some(count) => Ok(count),
            None => Err("Could not get reference count".to_string()),
        }
    }

    pub fn get_ptr_value(&self, ptr: &GCPointer<StoredData>) -> ExecResult<Arc<StoredData>> {
        let gc = self.state.read()
            .unwrap_or_else(|e| panic!("Could not get read lock on VM state, the lock is poisoned: {}", e));

        let get_result = gc.get(ptr);
        get_result.map(|value| value)
    }

    /// Drops a reference to a value from the VM's state, decrementing the reference count.
    pub fn drop_reference(&self, reference: &mut ValueReference) {
        let mut gc = self.state.write()
            .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

        if let Err(msg) = gc.drop_pointer(&mut reference.pointer) {
            panic!("Could not drop pointer: {}", msg);
        }
    }



    /// Fills a buffer with the given value
    pub fn execute_fill_buffer(&self, buffer: &ValueReference, value: StoredData) -> ExecResult<Vec<ValueReference>> {
        let mut gc = self.state.write()
            .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

        let fill_result = gc.fill_buffer(&buffer.pointer, value);

        if let Err(msg) = fill_result {
            return Err(msg);
        }

        return Ok(vec![])
    }

    

    /// Converts a value ref to a pointer (doesn't have a reference to the vm).
    /// Counts the new pointer so the ref count stays the same.
    pub fn counted_ptr_from_value_ref(&self, value_ref: ValueReference) -> ExecResult<PointerLive> {
        let mut gc = self.state.write()
            .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

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

/// Converts a pointer into a value reference that can be used by the VM's caller.
/// Counts the pointer if it is uncounted.
pub fn value_ref_from_ptr(mmu: Arc<MMU>, mut ptr: GCPointer<StoredData>) -> ExecResult<ValueReference> {
    // if the pointer is uncounted, we need to manually count it
    if !ptr.counted {
        let mut gc = mmu.state.write()
            .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

        match gc.count_pointer(&mut ptr) {
            Ok(_) => {},
            Err(e) => return Err(format!("Could not count pointer: {}", e)),
        }
    }

    let result = ValueReference::new(ptr, mmu);

    return Ok(result)
}

/// Gets the live type of stored data.
pub fn get_stored_type(mmu: Arc<MMU>, arg: &StoredData) -> ExecResult<TypeLive> {
    let type_ptr = match get_stored_type_ptr(mmu.clone(), arg) {
        Ok(ptr) => ptr,
        Err(msg) => return Err(msg),
    };

    let type_ref = match value_ref_from_ptr(mmu.clone(), type_ptr) {
        Ok(type_ref) => type_ref,
        Err(msg) => return Err(msg),
    };

    let type_value: Arc<StoredData> = match mmu.get_ref_value(&type_ref) {
        Ok(type_value) => type_value,
        Err(msg) => return Err(msg),
    };


    return match type_value.as_ref() {
        StoredData::TypeStored(type_live) => Ok(type_live.clone()),
        _ => Err("Could not get type of argument, type is a non-type value".to_string()),
    };
}

pub fn get_primitive_type(mmu: Arc<MMU>, type_live: &TypeLive) -> ExecResult<ValueReference> {
    match mmu.primitive_types.get(type_live) {
        Some(ptr) => value_ref_from_ptr(mmu.clone(), ptr.clone()).map(|value_ref| value_ref),
        None => Err(format!("Could not get pointer to primitive type {:?}", type_live)),
    }
}

/// Clones a value reference, incrementing the reference count.
pub fn clone_reference(mmu: Arc<MMU>, reference: &ValueReference) -> ExecResult<ValueReference> {
    let mut cloned_ptr = reference.pointer.clone();

    let mut gc = mmu.state.write()
        .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

    match gc.count_pointer(&mut cloned_ptr) {
        Ok(_) => {}
        Err(msg) => return Err(msg),
    }

    let new_reference = ValueReference::new(cloned_ptr, mmu.clone());

    return Ok(new_reference)
}

/// Stores a value in the VM's state, returning a reference to the stored value
pub fn store_value(mmu: Arc<MMU>, value: StoredData) -> ExecResult<Vec<ValueReference>> {
    // try to get write lock
    let mut gc = mmu.state.write()
        .unwrap_or_else(|e| panic!("Could not get write lock on VM state, the lock is poisoned: {}", e));

    // allocate value
    let ptr = match gc.allocate(value) {
        Ok(ptr) => ptr,
        Err(msg) => return Err(format!("Could not allocate value: {}", msg).to_string()),
    };

    // create value reference
    return match value_ref_from_ptr(mmu.clone(), ptr) {
        Ok(value_ref) => Ok(vec![value_ref]),
        Err(msg) => Err(format!("Could not create value reference: {}", msg).to_string()),
    }
}

pub fn execute_store(mmu: Arc<MMU>, operation: StoreOp) -> ExecResult<Vec<ValueReference>> {
    return match operation {
        StoreOp::CreateBuffer => store_value(mmu, StoredData::NullStored),
        StoreOp::FillBuffer(buffer, value) => mmu.execute_fill_buffer(buffer, value),
        StoreOp::StoreInt(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreFloat(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreString(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreBool(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StorePointer(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreList(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreDict(_) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreFunction(_, _, _) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreFunctionVal(_, _ ,_, _) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreFunctionOp(_, _, _) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreFunctionGraph(func, class_context) => store_function(mmu, &func, class_context),
        StoreOp::StoreCustomType(_, _) => store_value(mmu, operation.get_stored_data().unwrap()),
        StoreOp::StoreObject(_, _) => store_value(mmu, operation.get_stored_data().unwrap()),
    };
}