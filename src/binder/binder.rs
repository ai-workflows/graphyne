use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use crate::binder::intermediate::collection::Collection;
use crate::binder::intermediate::func::CollectionFunc;
use crate::binder::intermediate::r#const::{CCData, CollectionConst};
use crate::binder::intermediate::r#type::{CollectionType, CustomTypeDef};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::{ExecResult, Symbol, SymbolPath};
use crate::runtime::data::functions::op::FuncOpId;
use crate::runtime::data::live::{DictLive, FuncLive, FuncOpLive, FuncValLive, PointerLive, StaticRefLive, TypeLive};
use crate::runtime::data::stored::StoredData;


fn buffer_collection_property_group<T>(
    group: &Option<HashMap<Symbol, T>>,
    mut static_state: Arc<StaticState>,
    symbol_path: &SymbolPath,
    collection_refs: &mut DictLive
) -> ExecResult<()> {
    if let Some(properties) = group {
        for (name, _property) in properties {
            let mut path = symbol_path.clone();
            path.push(name.clone());
            let ref_ptr = static_state.buffer(&path)?;
            collection_refs.insert(name.clone(), ref_ptr);
        }
    }

    Ok(())
}


pub fn buffer_collection(
    mut static_state: Arc<StaticState>,
    symbol_path: &SymbolPath,
    value: &Collection
) -> ExecResult<PointerLive> {
    let mut res: DictLive = DictLive::new();

    buffer_collection_property_group(&value.functions, static_state.clone(), symbol_path, &mut res)?;
    buffer_collection_property_group(&value.constants, static_state.clone(), symbol_path, &mut res)?;
    buffer_collection_property_group(&value.types, static_state.clone(), symbol_path, &mut res)?;
    buffer_collection_property_group(&value.imports, static_state.clone(), symbol_path, &mut res)?;

    if let Some(collections) = &value.collections {
        for (name, sub_collection) in collections {
            let mut path = symbol_path.clone();
            path.push(name.clone());
            let ref_ptr = buffer_collection(static_state.clone(), &path, sub_collection)?;
            res.insert(name.clone(), ref_ptr);
        }
    }

    let cl_ptr = static_state.buffer(symbol_path)?;
    static_state.set(symbol_path, StoredData::DictStored(res))?;
    Ok(cl_ptr)
}

fn collection_const_data_to_stored(
    data: &CollectionConst,
    static_state: Arc<StaticState>
) -> ExecResult<StoredData> {
    cc_data_to_stored(&data.0, static_state)
}

fn cc_data_to_stored(
    data: &CCData,
    mut static_state: Arc<StaticState>
) -> ExecResult<StoredData> {
    match data {
        CCData::Int(val) => Ok(StoredData::IntStored(val.clone())),
        CCData::Float(val) => Ok(StoredData::FloatStored(val.clone())),
        CCData::String(val) => Ok(StoredData::StringStored(val.clone())),
        CCData::Bool(val) => Ok(StoredData::BoolStored(val.clone())),
        CCData::List(val) => {
            let mut res: Vec<PointerLive> = Vec::new();

            for child in val {
                let child_guid: SymbolPath = vec![Uuid::new_v4().to_string()];
                let static_ref_ptr: PointerLive = static_state.get_ptr_to_ref(&child_guid)?;

                static_state.set(&child_guid, cc_data_to_stored(child, static_state.clone())?)?;
                res.push(static_ref_ptr);
            }

            Ok(StoredData::ListStored(res))
        },
        CCData::Dict(val) => {
            let mut res: HashMap<Symbol, PointerLive> = HashMap::new();

            for (key, child) in val {
                let child_guid: SymbolPath = vec![Uuid::new_v4().to_string()];
                let static_ref_ptr: PointerLive = static_state.get_ptr_to_ref(&child_guid)?;

                static_state.set(&child_guid, cc_data_to_stored(child, static_state.clone())?)?;
                res.insert(key.clone(), static_ref_ptr);
            }

            Ok(StoredData::DictStored(res))
        },
        CCData::Null => Ok(StoredData::NullStored)
    }
}

fn type_def_to_live_type(
    type_def: &CustomTypeDef,
    static_state: Arc<StaticState>,
    type_symbol: &Symbol
) -> ExecResult<TypeLive> {
    let mut fields: Vec<(Symbol, PointerLive)> = Vec::new();
    for (field_symbol, field_type_const) in &type_def.0 {
        let type_ptr = match &field_type_const.0 {
            CollectionType::Any => static_state.get_primitive_type(&TypeLive::Dynamic),
            CollectionType::Null => static_state.get_primitive_type(&TypeLive::Null),
            CollectionType::Int => static_state.get_primitive_type(&TypeLive::Integer),
            CollectionType::Float => static_state.get_primitive_type(&TypeLive::Float),
            CollectionType::Str => static_state.get_primitive_type(&TypeLive::String),
            CollectionType::Bool => static_state.get_primitive_type(&TypeLive::Boolean),
            CollectionType::List => static_state.get_primitive_type(&TypeLive::List),
            CollectionType::Dict => static_state.get_primitive_type(&TypeLive::Dictionary),
            CollectionType::Type => static_state.get_primitive_type(&TypeLive::Type),
            CollectionType::Custom(type_symbol_path) => {
                let type_ref_ptr = static_state.get_ptr_to_ref(type_symbol_path)?;
                Some(type_ref_ptr)
            }
        };
        let type_ptr = match type_ptr {
            Some(type_ptr) => type_ptr,
            None => return Err(format!("Type reference not found for field {}", field_symbol)),
        };

        fields.push((field_symbol.clone(), type_ptr.clone()));
    }

    Ok(TypeLive::Custom(type_symbol.clone(), Uuid::new_v4().to_string(), fields))
}



fn cl_func_to_live_func(
    func: &CollectionFunc,
    func_symbol_path: &SymbolPath,
    mut static_state: Arc<StaticState>
) -> ExecResult<FuncLive> {
    // create buffers for the function values
    for val in &func.graph.values {
        let mut path: SymbolPath = func_symbol_path.clone();
        path.push(val.symbol.clone());
        static_state.buffer(&path)?;
    }

    // create the func ops and track the dependencies
    let mut val_deps: HashMap<Symbol, Vec<usize>> = HashMap::new();
    let mut ops: Vec<PointerLive> = Vec::with_capacity(func.graph.ops.len());

    for (i, op_node) in func.graph.ops.iter().enumerate() {
        let op_id: FuncOpId = "op_".to_string() + Uuid::new_v4().to_string().as_str();

        let input_ptrs: Vec<PointerLive> = op_node.input_vals.iter()
            .map(|input_symbol| {
                val_deps.entry(input_symbol.clone()).or_insert(Vec::new()).push(i);
                let mut path: SymbolPath = func_symbol_path.clone();
                path.push(input_symbol.clone());
                static_state.get_ptr_to_ref(&path)
            })
            .collect::<ExecResult<Vec<PointerLive>>>()?;

        let output_ptrs: Vec<PointerLive> = op_node.output_vals.iter()
            .map(|output_symbol| {
                let mut path: SymbolPath = func_symbol_path.clone();
                path.push(output_symbol.clone());
                static_state.get_ptr_to_ref(&path)
            })
            .collect::<ExecResult<Vec<PointerLive>>>()?;

        let func_op = FuncOpLive {
            guid: op_id,
            opcode: op_node.opcode.clone(),
            input_vals: input_ptrs,
            output_vals: output_ptrs,
        };

        let ptr = static_state.buffer(func_symbol_path)?;
        static_state.set(func_symbol_path, StoredData::FuncOpStored(func_op))?;

        ops.push(ptr);
    }

    // fill the buffers for the function values
    let mut constant_vals: Vec<PointerLive> = Vec::new();

    for val in &func.graph.values {
        let mut path: SymbolPath = func_symbol_path.clone();
        path.push(val.symbol.clone());

        let dependents: Vec<PointerLive> = val_deps.get(&val.symbol).cloned().unwrap_or_default()
            .iter()
            .map(|&op_id| ops[op_id].clone())
            .collect();

        let constant_ptr: Option<PointerLive> = match &val.constant {
            Some(constant_cc_data) => {
                let constant_stored_data: StoredData = cc_data_to_stored(constant_cc_data, static_state.clone())?;
                let constant_ptr: PointerLive = static_state.buffer(&path)?;
                static_state.set(&path, constant_stored_data)?;

                // add the func val to the constant_vals list
                let val_ptr: PointerLive = static_state.get_ptr_to_ref(&path)?;
                constant_vals.push(val_ptr);

                Some(constant_ptr)
            },
            None => None
        };

        let func_val = FuncValLive {
            symbol: Some(val.symbol.clone()),
            guid: Uuid::new_v4().to_string(),
            dependents,
            constant: constant_ptr,
            is_self: val.symbol == "outer"
        };

        static_state.set(&path, StoredData::FuncValStored(func_val))?;
    }

    // get the input and output values
    let input_vals: Vec<PointerLive> = func.graph.input_vals.iter()
        .map(|input_symbol| {
            let mut path: SymbolPath = func_symbol_path.clone();
            path.push(input_symbol.clone());
            static_state.get_ptr_to_ref(&path)
        })
        .collect::<ExecResult<Vec<PointerLive>>>()?;

    let output_vals: Vec<PointerLive> = func.graph.output_vals.iter()
        .map(|output_symbol| {
            let mut path: SymbolPath = func_symbol_path.clone();
            path.push(output_symbol.clone());
            static_state.get_ptr_to_ref(&path)
        })
        .collect::<ExecResult<Vec<PointerLive>>>()?;

    // create the function
    let func = FuncLive {
        symbol_path: Some(func_symbol_path.clone()),
        guid: Uuid::new_v4().to_string(),
        input_vals,
        output_vals,
        constant_vals
    };

    Ok(func)
}

pub fn fill_collection(
    static_state: Arc<StaticState>,
    symbol_path: &SymbolPath,
    value: &Collection
) -> ExecResult<()> {
    // store the constants at each location
    if let Some(constants) = &value.constants {
        for (name, constant) in constants {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            static_ref.set(collection_const_data_to_stored(constant, static_state.clone())?)
                .map_err(|_| format!("Error setting constant at path {:?}", path))?;
        }
    }

    // store the static reference to the imported collections at each import location
    if let Some(imports) = &value.imports {
        for (name, import_path) in imports {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let import_static_ref: StaticRefLive = static_state.get_ref(import_path)?;

            static_ref.set(StoredData::StaticRefStored(import_static_ref))
                .map_err(|_| format!("Error setting import at path {:?}", path))?;
        }
    }

    // store the custom types at each location
    if let Some(types) = &value.types {
        for (name, type_def) in types {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let live_type: TypeLive = type_def_to_live_type(type_def, static_state.clone(), &name)?;
            static_ref.set(StoredData::TypeStored(live_type))
                .map_err(|_| format!("Error setting type at path {:?}", path))?;
        }
    }

    // store the functions at each location
    if let Some(functions) = &value.functions {
        for (name, func) in functions {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let live_func: FuncLive = cl_func_to_live_func(func, &path, static_state.clone())?;
            static_ref.set(StoredData::FuncStored(live_func))
                .map_err(|_| format!("Error setting function at path {:?}", path))?;
        }
    }

    // fill the sub-collections
    if let Some(collections) = &value.collections {
        for (name, sub_collection) in collections {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            fill_collection(static_state.clone(), &path, sub_collection)?;
        }
    }

    Ok(())
}

pub fn bind_program(
    program: Collection,
    static_state: Arc<StaticState>,
    main_symbol: Option<Symbol>
) -> ExecResult<PointerLive> {
    let main_symbol: Symbol = main_symbol.unwrap_or_else(|| "main".to_string());
    let main_path: SymbolPath = vec![main_symbol.clone()];

    let main_ptr: PointerLive = buffer_collection(static_state.clone(), &main_path, &program)?;
    fill_collection(static_state.clone(), &main_path, &program)?;

    Ok(main_ptr)
}