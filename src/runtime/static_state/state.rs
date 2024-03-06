use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use crate::runtime::data::live::{PointerLive, StaticRefLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult, SymbolPath};

pub struct StaticState {
    static_refs: HashMap<SymbolPath, PointerLive>,
    primitive_types: Vec<PointerLive>
}

impl StaticState {
    pub fn new() -> Self {
        let mut res = StaticState {
            static_refs: HashMap::new(),
            primitive_types: Vec::new(),
        };

        // load in the primitive types
        res.allocate_primitive_types();

        res
    }
    
    fn allocate_primitive_types(&mut self) {
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Integer)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Float)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::String)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Boolean)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Pointer)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::List)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Dictionary)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Function)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::FunctionVal)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::FunctionOp)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Null)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Type)));
        self.primitive_types.push(PointerLive::new(StoredData::TypeStored(TypeLive::Dynamic)));
    }
    
    pub fn get_primitive_type(&self, t: &TypeLive) -> Option<PointerLive> {
        match t {
            TypeLive::Integer => Some(self.primitive_types[0].clone()),
            TypeLive::Float => Some(self.primitive_types[1].clone()),
            TypeLive::String => Some(self.primitive_types[2].clone()),
            TypeLive::Boolean => Some(self.primitive_types[3].clone()),
            TypeLive::Pointer => Some(self.primitive_types[4].clone()),
            TypeLive::List => Some(self.primitive_types[5].clone()),
            TypeLive::Dictionary => Some(self.primitive_types[6].clone()),
            TypeLive::Function => Some(self.primitive_types[7].clone()),
            TypeLive::FunctionVal => Some(self.primitive_types[8].clone()),
            TypeLive::FunctionOp => Some(self.primitive_types[9].clone()),
            TypeLive::Null => Some(self.primitive_types[10].clone()),
            TypeLive::Type => Some(self.primitive_types[11].clone()),
            TypeLive::Dynamic => Some(self.primitive_types[12].clone()),
            _ => None,
        }
    }

    /// Creates a new static reference at the given symbol path with no value.
    pub fn buffer(&mut self, symbol_path: &SymbolPath) -> ExecResult<PointerLive> {
        if self.static_refs.contains_key(symbol_path) {
            return Err(format!("Static reference already exists at path {:?}", symbol_path));
        }

        let ptr = PointerLive::new(StoredData::StaticRefStored(Arc::new(OnceLock::new())));
        self.static_refs.insert(symbol_path.clone(), ptr.clone());
        Ok(ptr)
    }

    /// Sets the value of a static reference at the given symbol path.
    pub fn set(&mut self, symbol_path: &SymbolPath, value: StoredData) -> ExecResult<()> {
        let static_ref: StaticRefLive = self.get_ref(symbol_path)?;

        static_ref.set(value)
            .map_err(|_| format!("Error setting static reference at path {:?}", symbol_path))
    }

    // pub fn get_deref_val(&self, symbol_path: &SymbolPath) -> ExecResult<&StoredData> {
    //     let static_ref: StaticRefLive = self.get_ref(symbol_path)?;
    //
    //     match static_ref.get() {
    //         Some(data) => Ok(data),
    //         None => Err(format!("Static reference at path {:?} is not initialized.", symbol_path)),
    //     }
    // }

    pub fn get_ref(&self, symbol_path: &SymbolPath) -> ExecResult<StaticRefLive> {
        match self.static_refs.get(symbol_path) {
            Some(ptr) => Ok(ptr.as_ref().as_static_ref()?.clone()),
            None => Err(format!("Static reference not found at path {:?}", symbol_path)),
        }
    }

    pub fn get_ptr_to_ref(&self, symbol_path: &SymbolPath) -> ExecResult<PointerLive> {
        match self.static_refs.get(symbol_path) {
            Some(ptr) => Ok(ptr.clone()),
            None => Err(format!("Static reference not found at path {:?}", symbol_path)),
        }
    }
}