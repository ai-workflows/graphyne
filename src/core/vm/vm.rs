use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::live::PointerLive;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::{GarbageCollector};
use crate::core::vm::ops::Operation;
use crate::core::vm::store::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;



macro_rules! allocate_primitive_type {
    ($state:ident, $prims:ident, $type:path) => {
        match $state.allocate(StoredData::TypeStored($type)) {
            Ok(ptr) => $prims.insert($type, ptr),
            Err(msg) => return Err(msg),
        };
    };
}

#[derive(Debug)]
pub struct VM {
    pub state: Arc<RwLock<GarbageCollector<StoredData>>>,
    pub thread_pool: rayon::ThreadPool,
    pub primitive_types: HashMap<TypeLive, PointerLive>,
}

impl VM {
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

    pub fn get_primitive_type(&self, type_live: &TypeLive) -> ExecResult<ValueReference> {
        match self.primitive_types.get(type_live) {
            Some(ptr) => self.value_ref_from_ptr(ptr.clone()).map(|value_ref| value_ref),
            None => Err(format!("Could not get pointer to primitive type {:?}", type_live)),
        }
    }

    pub fn new(num_threads: usize) -> Self {
        let state = GarbageCollector::new();

        let mut res = VM {
            state: Arc::new(RwLock::new(state)),
            thread_pool: rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap(),
            primitive_types: HashMap::new(),
        };

        // load in the primitive types
        match res.allocate_primitive_types() {
            Ok(types) => types,
            Err(msg) => panic!("Could not allocate primitive types: {}", msg),
        };

        res
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

    pub fn execute_store(&self, operation: StoreOp) -> ExecResult<Vec<ValueReference>> {
        return match operation {
            StoreOp::CreateBuffer => self.store_value(StoredData::NullStored),
            StoreOp::StoreInt(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFloat(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreString(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreBool(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StorePointer(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreList(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreDict(_) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunction(_, _, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionVal(_, _ ,_, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionOp(_, _, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreFunctionGraph(func, class_context) => self.store_function(&func, class_context),
            StoreOp::StoreCustomType(_, _) => self.store_value(operation.get_stored_data().unwrap()),
            StoreOp::StoreObject(_, _) => self.store_value(operation.get_stored_data().unwrap()),
        };
    }

    pub fn execute_op(&self, operation: Operation) -> ExecResult<Vec<ValueReference>> {
        match operation {
            Operation::SetBuffer(buffer, value) => self.execute_fill_buffer(buffer, value),

            Operation::TypeOf(arg) => self.execute_type_of(arg),
            Operation::AsInt(arg) => self.execute_as_int(arg),
            Operation::AsFloat(arg) => self.execute_as_float(arg),
            Operation::AsString(arg) => self.execute_as_string(arg),
            Operation::AsBool(arg) => self.execute_as_bool(arg),
            Operation::AsPointer(arg) => self.execute_as_pointer(arg),
            Operation::AsList(arg) => self.execute_as_list(arg),
            Operation::AsDictionary(arg) => self.execute_as_dict(arg),
            Operation::AsType(arg) => self.execute_as_type(arg),

            Operation::Add(lhs, rhs) => self.execute_add(lhs, rhs),
            Operation::Sub(lhs, rhs) => self.execute_sub(lhs, rhs),
            Operation::Mul(lhs, rhs) => self.execute_mul(lhs, rhs),
            Operation::Div(lhs, rhs) => self.execute_div(lhs, rhs),
            Operation::Mod(lhs, rhs) => self.execute_mod(lhs, rhs),
            Operation::Pow(lhs, rhs) => self.execute_pow(lhs, rhs),

            Operation::Length(list) => self.execute_length(list),
            Operation::GetItem(list, index) => self.execute_get_item(list, index),
            Operation::SetItem(list, index, value) => self.execute_set_item(list, index, value),
            Operation::Push(list, value) => self.execute_push(list, value),
            Operation::Remove(list, index) => self.execute_remove(list, index),

            Operation::If(condition, then, otherwise) => self.execute_if(condition, then, otherwise),
            Operation::Not(arg) => self.execute_not(arg),
            Operation::And(lhs, rhs) => self.execute_and(lhs, rhs),
            Operation::Or(lhs, rhs) => self.execute_or(lhs, rhs),
            Operation::Equal(lhs, rhs) => self.execute_equal(lhs, rhs),
            Operation::LessThan(lhs, rhs) => self.execute_less_than(lhs, rhs),
            Operation::GreaterThan(lhs, rhs) => self.execute_greater_than(lhs, rhs),
            Operation::IsNull(arg) => self.execute_is_null(arg),

            Operation::Call(func, args) => self.execute_call(func, args),
            Operation::Map(func, list) => self.map(func, list),
            Operation::Reduce(func, list, initial) => self.handle_reduce(func, list, initial),
            Operation::Filter(func, list) => self.filter(func, list),

            Operation::Init(obj_type, args) => self.execute_init(obj_type, args),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::vm::store::store_op::StoreOp;
    use crate::core::vm::VM;
    use crate::core::data::live::{LiveData};
    use crate::core::vm::value_ref::ValueReference;

    #[test]
    fn test_store_pointer() {
        let mut vm = VM::new(4);

        // test storing pointers
        test_store_pointer_helper(&mut vm, "hello");
        assert_eq!(vm.object_count(), 0);

        test_store_pointer_helper(&mut vm, "");
        assert_eq!(vm.object_count(), 0);

    }

    fn test_store_pointer_helper2<'a>(vm: &'a mut VM, value: &str) -> ValueReference<'a> {
        let st_results: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(value.to_string())).unwrap();
        let ptr = st_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 1);

        let st_results_meta: Vec<ValueReference> = vm.execute_store(StoreOp::StorePointer(ptr)).unwrap();
        let meta_ptr = st_results_meta.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr).unwrap(), 2);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        println!("{:?}", vm.state);
        //
        // println!("{:?}", meta_ptr.get().unwrap().as_live().as_pointer().unwrap());

        assert_eq!(vm.get_ref_value(meta_ptr).unwrap().as_live().as_pointer(), Some(Ok(ptr.pointer.clone())));

        // There should be 2 objects in the VM: the string and the pointer
        assert_eq!(vm.object_count(), 2);

        // println!("{:?}", vm.state);

        let result = ptr.clone();

        // there should now be three references to the pointer: ptr, the pointer in the meta, and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 3);
        assert_eq!(vm.ref_count(meta_ptr).unwrap(), 1);

        drop(st_results_meta);

        // there should now be two references to the pointer: ptr and the result
        assert_eq!(vm.ref_count(ptr).unwrap(), 2);

        drop(st_results);

        // there should now be one reference to the pointer: the result
        assert_eq!(vm.ref_count(&result).unwrap(), 1);

        return result;
    }

    fn test_store_pointer_helper(vm: &mut VM, value: &str) {
        vm.reset();

        let ptr = test_store_pointer_helper2(vm, value);

        // println!("{:?}", ptr);

        assert_eq!(ptr.vm.ref_count(&ptr).unwrap(), 1);

        // There should be 1 object in the VM: the pointer
        assert_eq!(ptr.vm.object_count(), 1);
    }
}