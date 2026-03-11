use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::binder::intermediate::collection::Collection;
use crate::binder::intermediate::func::{CollectionFunc};
use crate::binder::intermediate::r#const::{CCData, CollectionConst};
use crate::binder::intermediate::r#type::{CollectionType, CustomTypeDef};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::{ExecResult, Symbol, SymbolPath};
use crate::runtime::data::functions::OpCode;
use crate::runtime::data::functions::func::{FuncOp, FuncLive, FuncVal};
use crate::runtime::data::live::{DictLive, PointerLive, StaticRefLive, TypeLive};
use crate::runtime::data::stored::StoredData;


fn validate_collection_symbol_uniqueness(
    symbol_path: &SymbolPath,
    value: &Collection,
) -> ExecResult<()> {
    let mut seen: HashMap<&str, &str> = HashMap::new();

    macro_rules! check_group {
        ($group_name:literal, $group:expr) => {
            if let Some(entries) = $group {
                for symbol in entries.keys() {
                    if let Some(existing_group) = seen.insert(symbol.as_str(), $group_name) {
                        return Err(format!(
                            "Duplicate collection symbol '{}' in {:?} across {} and {}",
                            symbol,
                            symbol_path,
                            existing_group,
                            $group_name
                        ));
                    }
                }
            }
        };
    }

    check_group!("functions", value.functions.as_ref());
    check_group!("constants", value.constants.as_ref());
    check_group!("types", value.types.as_ref());
    check_group!("imports", value.imports.as_ref());
    check_group!("collections", value.collections.as_ref());

    if let Some(collections) = &value.collections {
        for (name, sub_collection) in collections {
            let mut sub_path = symbol_path.clone();
            sub_path.push(name.clone());
            validate_collection_symbol_uniqueness(&sub_path, sub_collection)?;
        }
    }

    Ok(())
}

fn buffer_collection_property_group<T>(
    group: &Option<HashMap<Symbol, T>>,
    static_state: &mut StaticState,
    symbol_path: &SymbolPath,
    collection_refs: &mut DictLive
) -> ExecResult<()> {
    if let Some(properties) = group {
        for name in properties.keys() {
            let mut path = symbol_path.clone();
            path.push(name.clone());
            let ref_ptr = static_state.buffer(&path)?;
            collection_refs.insert(name.clone(), ref_ptr);
        }
    }

    Ok(())
}


pub fn buffer_collection(
    static_state: &mut StaticState,
    symbol_path: &SymbolPath,
    value: &Collection
) -> ExecResult<PointerLive> {
    let mut res: DictLive = DictLive::new();

    buffer_collection_property_group(&value.functions, static_state, symbol_path, &mut res)?;
    buffer_collection_property_group(&value.constants, static_state, symbol_path, &mut res)?;
    buffer_collection_property_group(&value.types, static_state, symbol_path, &mut res)?;
    buffer_collection_property_group(&value.imports, static_state, symbol_path, &mut res)?;

    if let Some(collections) = &value.collections {
        for (name, sub_collection) in collections {
            let mut path = symbol_path.clone();
            path.push(name.clone());
            let ref_ptr = buffer_collection(static_state, &path, sub_collection)?;
            res.insert(name.clone(), ref_ptr);
        }
    }

    let cl_ptr = static_state.buffer(symbol_path)?;
    static_state.set(symbol_path, StoredData::DictStored(res))?;
    Ok(cl_ptr)
}

fn collection_const_data_to_stored(
    data: &CollectionConst,
    static_state: &mut StaticState
) -> ExecResult<StoredData> {
    cc_data_to_stored(&data.0, static_state)
}

fn cc_data_to_stored(
    data: &CCData,
    static_state: &mut StaticState
) -> ExecResult<StoredData> {
    match data {
        CCData::Int(val) => Ok(StoredData::IntStored(*val)),
        CCData::Float(val) => Ok(StoredData::FloatStored(*val)),
        CCData::String(val) => Ok(StoredData::StringStored(val.clone())),
        CCData::Bool(val) => Ok(StoredData::BoolStored(*val)),
        CCData::List(val) => {
            let mut res: Vec<PointerLive> = Vec::new();

            for child in val {
                let child_guid: SymbolPath = vec![Uuid::new_v4().to_string()];
                let static_ref_ptr: PointerLive = static_state.buffer(&child_guid)?;

                let cc_data_stored: StoredData = cc_data_to_stored(child, static_state)?;

                static_state.set(&child_guid, cc_data_stored)?;
                res.push(static_ref_ptr);
            }

            Ok(StoredData::ListStored(res))
        },
        CCData::Dict(val) => {
            let mut res: HashMap<Symbol, PointerLive> = HashMap::new();

            for (key, child) in val {
                let child_guid: SymbolPath = vec![Uuid::new_v4().to_string()];
                let static_ref_ptr: PointerLive = static_state.buffer(&child_guid)?;

                let cc_data_stored: StoredData = cc_data_to_stored(child, static_state)?;

                static_state.set(&child_guid, cc_data_stored)?;
                res.insert(key.clone(), static_ref_ptr);
            }

            Ok(StoredData::DictStored(res))
        },
        CCData::Null => Ok(StoredData::NullStored)
    }
}

fn validate_custom_type_fields(
    type_symbol: &Symbol,
    type_def: &CustomTypeDef,
    collection_symbol_path: &SymbolPath,
) -> ExecResult<()> {
    let mut seen: HashSet<&str> = HashSet::new();

    for (field_symbol, _) in &type_def.0 {
        if !seen.insert(field_symbol.as_str()) {
            return Err(format!(
                "Duplicate field '{}' in type '{}' at {:?}",
                field_symbol,
                type_symbol,
                collection_symbol_path
            ));
        }
    }

    Ok(())
}

fn type_def_to_live_type(
    type_def: &CustomTypeDef,
    static_state: &mut StaticState,
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
            CollectionType::Pointer => static_state.get_primitive_type(&TypeLive::Pointer),
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


fn validate_unique_symbols(symbols: &[Symbol], group_name: &str, func_symbol_path: &SymbolPath) -> ExecResult<()> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for symbol in symbols {
        let count = seen.entry(symbol.as_str()).or_default();
        *count += 1;
        if *count > 1 {
            return Err(format!(
                "Duplicate {} symbol '{}' in function {:?}",
                group_name,
                symbol,
                func_symbol_path
            ));
        }
    }

    Ok(())
}

fn validate_op_arity(op_node: &crate::binder::intermediate::func::FunctionOpNode, func_symbol_path: &SymbolPath) -> ExecResult<()> {
    let (expected_inputs, expected_outputs) = match op_node.opcode {
        OpCode::TypeOf
        | OpCode::AsInt
        | OpCode::AsFloat
        | OpCode::AsString
        | OpCode::AsBool
        | OpCode::AsPointer
        | OpCode::AsList
        | OpCode::AsDictionary
        | OpCode::AsType
        | OpCode::Not
        | OpCode::IsNull
        | OpCode::Length => (Some(1), Some(1)),
        OpCode::And
        | OpCode::Or
        | OpCode::Equal
        | OpCode::LessThan
        | OpCode::GreaterThan
        | OpCode::Get
        | OpCode::Push
        | OpCode::Remove
        | OpCode::Add
        | OpCode::Sub
        | OpCode::Mul
        | OpCode::Div
        | OpCode::Mod
        | OpCode::Pow
        | OpCode::Map
        | OpCode::Filter => (Some(2), Some(1)),
        OpCode::If
        | OpCode::Set
        | OpCode::Reduce => (Some(3), Some(1)),
        OpCode::Static => (Some(1), Some(1)),
        OpCode::Call | OpCode::Init => (None, None),
    };

    if let Some(expected_inputs) = expected_inputs {
        if op_node.input_vals.len() != expected_inputs {
            return Err(format!(
                "Opcode {} in function {:?} expects {} inputs but received {}",
                op_node.opcode,
                func_symbol_path,
                expected_inputs,
                op_node.input_vals.len()
            ));
        }
    } else {
        let min_inputs = match op_node.opcode {
            OpCode::Call | OpCode::Init => 1,
            _ => 0,
        };

        if op_node.input_vals.len() < min_inputs {
            return Err(format!(
                "Opcode {} in function {:?} requires at least {} inputs but received {}",
                op_node.opcode,
                func_symbol_path,
                min_inputs,
                op_node.input_vals.len()
            ));
        }
    }

    if let Some(expected_outputs) = expected_outputs {
        if op_node.output_vals.len() != expected_outputs {
            return Err(format!(
                "Opcode {} in function {:?} expects {} outputs but received {}",
                op_node.opcode,
                func_symbol_path,
                expected_outputs,
                op_node.output_vals.len()
            ));
        }
    }

    Ok(())
}

fn validate_graph_value_declarations(
    func: &CollectionFunc,
    func_symbol_path: &SymbolPath,
) -> ExecResult<HashMap<Symbol, usize>> {
    let mut symbol_idxs: HashMap<Symbol, usize> = HashMap::new();

    for (i, val) in func.graph.values.iter().enumerate() {
        if symbol_idxs.insert(val.symbol.clone(), i).is_some() {
            return Err(format!(
                "Duplicate value symbol '{}' in function {:?}",
                val.symbol,
                func_symbol_path
            ));
        }
    }

    for input_symbol in &func.graph.input_vals {
        if !symbol_idxs.contains_key(input_symbol) {
            return Err(format!(
                "Input symbol '{}' is not declared in values for function {:?}",
                input_symbol,
                func_symbol_path
            ));
        }
    }

    for output_symbol in &func.graph.output_vals {
        if !symbol_idxs.contains_key(output_symbol) {
            return Err(format!(
                "Output symbol '{}' is not declared in values for function {:?}",
                output_symbol,
                func_symbol_path
            ));
        }
    }

    for op_node in &func.graph.ops {
        if op_node.opcode != OpCode::Static {
            for input_symbol in &op_node.input_vals {
                if !symbol_idxs.contains_key(input_symbol) {
                    return Err(format!(
                        "Op input symbol '{}' is not declared in values for function {:?}",
                        input_symbol,
                        func_symbol_path
                    ));
                }
            }
        }

        for output_symbol in &op_node.output_vals {
            if !symbol_idxs.contains_key(output_symbol) {
                return Err(format!(
                    "Op output symbol '{}' is not declared in values for function {:?}",
                    output_symbol,
                    func_symbol_path
                ));
            }
        }
    }

    Ok(symbol_idxs)
}

fn validate_single_assignment(
    func: &CollectionFunc,
    func_symbol_path: &SymbolPath,
) -> ExecResult<()> {
    let mut write_counts: HashMap<&str, usize> = HashMap::new();

    for val in &func.graph.values {
        if val.constant.is_some() {
            *write_counts.entry(val.symbol.as_str()).or_default() += 1;
        }
    }

    for input_symbol in &func.graph.input_vals {
        *write_counts.entry(input_symbol.as_str()).or_default() += 1;
    }

    for op_node in &func.graph.ops {
        for output_symbol in &op_node.output_vals {
            *write_counts.entry(output_symbol.as_str()).or_default() += 1;
        }
    }

    for (symbol, count) in write_counts {
        if count > 1 {
            return Err(format!(
                "Value '{}' in function {:?} is assigned {} times",
                symbol,
                func_symbol_path,
                count
            ));
        }
    }

    Ok(())
}

fn get_local_function_signature(
    callee_symbol: &str,
    sibling_functions: Option<&HashMap<Symbol, CollectionFunc>>,
) -> Option<(usize, usize)> {
    let sibling_functions = sibling_functions?;
    let callee = sibling_functions.get(callee_symbol)?;
    Some((callee.graph.input_vals.len(), callee.graph.output_vals.len()))
}

fn get_local_function_output_constant<'a>(
    callee_symbol: &str,
    sibling_functions: Option<&'a HashMap<Symbol, CollectionFunc>>,
    output_idx: usize,
) -> Option<&'a CCData> {
    let sibling_functions = sibling_functions?;
    let callee = sibling_functions.get(callee_symbol)?;
    let output_symbol = callee.graph.output_vals.get(output_idx)?;
    callee.graph.values.iter()
        .find(|value| &value.symbol == output_symbol)
        .and_then(|value| value.constant.as_ref())
}

fn validate_local_call_cycles(
    sibling_functions: Option<&HashMap<Symbol, CollectionFunc>>,
    collection_symbol_path: &SymbolPath,
) -> ExecResult<()> {
    let Some(sibling_functions) = sibling_functions else {
        return Ok(());
    };

    let mut call_graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for (func_name, func) in sibling_functions {
        let callees = func.graph.ops.iter()
            .filter(|op_node| op_node.opcode == OpCode::Call)
            .filter_map(|op_node| op_node.input_vals.first())
            .filter(|callee_symbol| sibling_functions.contains_key(*callee_symbol))
            .map(|callee_symbol| callee_symbol.as_str())
            .collect();
        call_graph.insert(func_name.as_str(), callees);
    }

    let mut visiting: HashSet<&str> = HashSet::new();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();

    fn visit<'a>(
        func_name: &'a str,
        call_graph: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
        stack: &mut Vec<&'a str>,
        collection_symbol_path: &SymbolPath,
    ) -> ExecResult<()> {
        if visited.contains(func_name) {
            return Ok(());
        }

        if !visiting.insert(func_name) {
            let cycle_start = stack.iter().position(|name| *name == func_name).unwrap_or(0);
            let mut cycle: Vec<&str> = stack[cycle_start..].to_vec();
            cycle.push(func_name);
            let cycle = cycle.join(" -> ");
            return Err(format!(
                "Recursive call cycle detected in {:?}: {}",
                collection_symbol_path,
                cycle
            ));
        }

        stack.push(func_name);

        if let Some(callees) = call_graph.get(func_name) {
            for callee in callees {
                visit(callee, call_graph, visiting, visited, stack, collection_symbol_path)?;
            }
        }

        stack.pop();
        visiting.remove(func_name);
        visited.insert(func_name);
        Ok(())
    }

    for func_name in sibling_functions.keys() {
        visit(
            func_name.as_str(),
            &call_graph,
            &mut visiting,
            &mut visited,
            &mut stack,
            collection_symbol_path,
        )?;
    }

    Ok(())
}

enum CollectionTarget<'a> {
    Function(&'a CollectionFunc),
    Type(&'a CustomTypeDef),
    Import(&'a SymbolPath),
    Other,
}

enum ImportedTarget<'a> {
    Function(&'a CollectionFunc),
    Type(&'a CustomTypeDef),
    Other,
}

fn get_collection_target<'a>(
    root_collection: &'a Collection,
    root_symbol_path: &SymbolPath,
    resolved_path: &SymbolPath,
) -> Option<CollectionTarget<'a>> {
    if !resolved_path.starts_with(root_symbol_path) {
        return None;
    }

    let relative_path = &resolved_path[root_symbol_path.len()..];
    if relative_path.is_empty() {
        return Some(CollectionTarget::Other);
    }

    let (target_symbol, collection_path) = relative_path.split_last()?;
    let mut collection = root_collection;

    for segment in collection_path {
        collection = collection.collections.as_ref()?.get(segment)?;
    }

    if let Some(functions) = &collection.functions {
        if let Some(func) = functions.get(target_symbol) {
            return Some(CollectionTarget::Function(func));
        }
    }

    if let Some(types) = &collection.types {
        if let Some(type_def) = types.get(target_symbol) {
            return Some(CollectionTarget::Type(type_def));
        }
    }

    if let Some(imports) = &collection.imports {
        if let Some(import_path) = imports.get(target_symbol) {
            return Some(CollectionTarget::Import(import_path));
        }
    }

    if collection.constants.as_ref().is_some_and(|constants| constants.contains_key(target_symbol))
        || collection.collections.as_ref().is_some_and(|collections| collections.contains_key(target_symbol))
    {
        return Some(CollectionTarget::Other);
    }

    None
}

fn resolve_imported_target<'a>(
    symbol: &str,
    sibling_imports: Option<&HashMap<Symbol, SymbolPath>>,
    root_collection: &'a Collection,
    root_symbol_path: &SymbolPath,
) -> Option<ImportedTarget<'a>> {
    let sibling_imports = sibling_imports?;
    let import_path = sibling_imports.get(symbol)?;
    let mut resolved_path = resolve_import_path(root_symbol_path, import_path);
    let mut visited: HashSet<SymbolPath> = HashSet::new();

    loop {
        if !visited.insert(resolved_path.clone()) {
            return None;
        }

        match get_collection_target(root_collection, root_symbol_path, &resolved_path)? {
            CollectionTarget::Function(func) => return Some(ImportedTarget::Function(func)),
            CollectionTarget::Type(type_def) => return Some(ImportedTarget::Type(type_def)),
            CollectionTarget::Import(import_path) => {
                resolved_path = resolve_import_path(root_symbol_path, import_path);
            }
            CollectionTarget::Other => return Some(ImportedTarget::Other),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cl_func_to_live_func(
    func: &CollectionFunc,
    func_symbol_path: &SymbolPath,
    sibling_functions: Option<&HashMap<Symbol, CollectionFunc>>,
    sibling_types: Option<&HashMap<Symbol, CustomTypeDef>>,
    sibling_imports: Option<&HashMap<Symbol, SymbolPath>>,
    root_collection: &Collection,
    root_symbol_path: &SymbolPath,
    static_state: &mut StaticState
) -> ExecResult<FuncLive> {
    validate_unique_symbols(&func.graph.input_vals, "input", func_symbol_path)?;
    validate_unique_symbols(&func.graph.output_vals, "output", func_symbol_path)?;

    let symbol_idxs = validate_graph_value_declarations(func, func_symbol_path)?;
    validate_single_assignment(func, func_symbol_path)?;

    let mut val_deps: HashMap<Symbol, Vec<usize>> = HashMap::new();
    let mut val_as_args: HashMap<Symbol, Vec<(usize, usize)>> = HashMap::new();
    let mut ops: Vec<FuncOp> = Vec::with_capacity(func.graph.ops.len());
    let mut call_ops: Vec<usize> = Vec::new();

    let mut static_val_constants: HashMap<Symbol, PointerLive> = HashMap::new();

    for (i, op_node) in func.graph.ops.iter().enumerate() {
        validate_op_arity(op_node, func_symbol_path)?;

        if op_node.opcode == OpCode::Static {
            // if the op is static, get the static ref and store it in the static state
            let mut static_path: SymbolPath = func_symbol_path.iter()
                .take(func_symbol_path.len() - 1).cloned().collect();
            static_path.extend(op_node.input_vals.iter().cloned());

            let static_ref: StaticRefLive = static_state.get_ref(&static_path).map_err(|_| {
                format!(
                    "Static reference {:?} used by function {:?} is not declared",
                    static_path,
                    func_symbol_path
                )
            })?;

            let output_symbol: Symbol = op_node.output_vals.first().cloned()
                .expect("Static op must have an output value");

            let mut static_val_path: SymbolPath = func_symbol_path.clone();
            static_val_path.push(output_symbol.clone());

            let mut static_val_const_path: SymbolPath = static_val_path.clone();
            static_val_const_path.push("constant".to_string());

            let static_val_const_ptr = static_state.buffer(&static_val_const_path)?;
            static_state.set(&static_val_const_path, StoredData::StaticRefStored(static_ref))?;

            static_val_constants.insert(output_symbol.clone(), static_val_const_ptr);

            continue;
        }

        let input_vals: Vec<usize> = op_node.input_vals.iter()
            .map(|input_symbol| {
                val_deps.entry(input_symbol.clone()).or_default().push(i - static_val_constants.len());
                match symbol_idxs.get(input_symbol).cloned() {
                    Some(idx) => Ok(idx),
                    None => Err(format!("Input value not found for op {}: {}", i, input_symbol))
                }
            })
            .collect::<ExecResult<Vec<usize>>>().map_err(|e| format!("Error getting input indices for op {}: {}", i, e))?;

        let output_vals: Vec<usize> = op_node.output_vals.iter()
            .map(|output_symbol| {
                match symbol_idxs.get(output_symbol).cloned() {
                    Some(idx) => Ok(idx),
                    None => Err(format!("Output value not found for op {}: {}", i, output_symbol))
                }
            })
            .collect::<ExecResult<Vec<usize>>>().map_err(|e| format!("Error getting output indices for op {}: {}", i, e))?;

        let func_op = FuncOp {
            index: i - static_val_constants.len(),
            opcode: op_node.opcode,
            input_vals,
            output_vals,
        };

        if matches!(op_node.opcode, OpCode::Call | OpCode::Map | OpCode::Filter | OpCode::Reduce) {
            let target_kind = match op_node.opcode {
                OpCode::Call => "Call",
                OpCode::Map => "Map",
                OpCode::Filter => "Filter",
                OpCode::Reduce => "Reduce",
                _ => unreachable!(),
            };

            if let Some(callee_symbol) = op_node.input_vals.first() {
                let imported_target = resolve_imported_target(
                    callee_symbol,
                    sibling_imports,
                    root_collection,
                    root_symbol_path,
                );

                let local_signature = get_local_function_signature(callee_symbol, sibling_functions);
                let imported_signature = match imported_target {
                    Some(ImportedTarget::Function(func)) => {
                        Some((func.graph.input_vals.len(), func.graph.output_vals.len()))
                    }
                    _ => None,
                };

                if op_node.opcode == OpCode::Call {
                    if let Some((callee_inputs, callee_outputs)) = local_signature.or(imported_signature) {
                        let expected_inputs = callee_inputs + 1;
                        if op_node.input_vals.len() != expected_inputs {
                            return Err(format!(
                                "Call to '{}' in function {:?} expects {} inputs but received {}",
                                callee_symbol,
                                func_symbol_path,
                                expected_inputs,
                                op_node.input_vals.len()
                            ));
                        }

                        if op_node.output_vals.len() != callee_outputs {
                            return Err(format!(
                                "Call to '{}' in function {:?} expects {} outputs but received {}",
                                callee_symbol,
                                func_symbol_path,
                                callee_outputs,
                                op_node.output_vals.len()
                            ));
                        }
                    }
                } else if let Some((callee_inputs, callee_outputs)) = local_signature.or(imported_signature) {
                    let expected_inputs = match op_node.opcode {
                        OpCode::Map | OpCode::Filter => 1,
                        OpCode::Reduce => 2,
                        _ => unreachable!(),
                    };

                    if callee_inputs != expected_inputs {
                        return Err(format!(
                            "{} target '{}' in function {:?} must accept exactly {} inputs but accepts {}",
                            target_kind,
                            callee_symbol,
                            func_symbol_path,
                            expected_inputs,
                            callee_inputs
                        ));
                    }

                    if callee_outputs != 1 {
                        return Err(format!(
                            "{} target '{}' in function {:?} must produce exactly 1 output but produces {}",
                            target_kind,
                            callee_symbol,
                            func_symbol_path,
                            callee_outputs
                        ));
                    }

                    if op_node.opcode == OpCode::Filter {
                        if let Some(output_constant) = get_local_function_output_constant(callee_symbol, sibling_functions, 0) {
                            if !matches!(output_constant, CCData::Bool(_)) {
                                return Err(format!(
                                    "Filter target '{}' in function {:?} must produce a bool output",
                                    callee_symbol,
                                    func_symbol_path
                                ));
                            }
                        }
                    }
                }

                if let Some(callee_idx) = symbol_idxs.get(callee_symbol) {
                    let callee_val = &func.graph.values[*callee_idx];
                    if callee_val.constant.is_some() {
                        return Err(format!(
                            "{} target '{}' in function {:?} is not a function",
                            target_kind,
                            callee_symbol,
                            func_symbol_path
                        ));
                    }
                }

                if matches!(imported_target, Some(ImportedTarget::Other) | Some(ImportedTarget::Type(_))) {
                    return Err(format!(
                        "{} target '{}' in function {:?} is not a function",
                        target_kind,
                        callee_symbol,
                        func_symbol_path
                    ));
                }
            }
        }

        if op_node.opcode == OpCode::Call {
            for (arg_idx, arg_symbol) in op_node.input_vals.iter().enumerate() {
                val_as_args.entry(arg_symbol.clone()).or_default().push((call_ops.len(), arg_idx));
            }
            call_ops.push(func_op.index);
        }

        if op_node.opcode == OpCode::Init {
            if op_node.output_vals.len() != 1 {
                return Err(format!(
                    "Opcode init in function {:?} expects 1 outputs but received {}",
                    func_symbol_path,
                    op_node.output_vals.len()
                ));
            }

            if let Some(type_symbol) = op_node.input_vals.first() {
                let imported_target = resolve_imported_target(
                    type_symbol,
                    sibling_imports,
                    root_collection,
                    root_symbol_path,
                );

                if let Some(type_idx) = symbol_idxs.get(type_symbol) {
                    let type_val = &func.graph.values[*type_idx];
                    if type_val.constant.is_some() {
                        return Err(format!(
                            "Init target '{}' in function {:?} is not a custom type",
                            type_symbol,
                            func_symbol_path
                        ));
                    }
                }

                let local_type_field_count = sibling_types
                    .and_then(|types| types.get(type_symbol))
                    .map(|type_def| type_def.0.len());
                let imported_type_field_count = match imported_target {
                    Some(ImportedTarget::Type(type_def)) => Some(type_def.0.len()),
                    _ => None,
                };

                if let Some(expected_init_args) = local_type_field_count.or(imported_type_field_count) {
                    let provided_init_args = op_node.input_vals.len() - 1;
                    if provided_init_args != expected_init_args {
                        return Err(format!(
                            "Init of '{}' in function {:?} expects {} fields but received {}",
                            type_symbol,
                            func_symbol_path,
                            expected_init_args,
                            provided_init_args
                        ));
                    }
                }

                if matches!(imported_target, Some(ImportedTarget::Function(_)) | Some(ImportedTarget::Other)) {
                    return Err(format!(
                        "Init target '{}' in function {:?} is not a custom type",
                        type_symbol,
                        func_symbol_path
                    ));
                }
            }
        }

        ops.push(func_op);
    }

    // get the input/output indices
    let mut output_idxs: HashMap<Symbol, usize> = HashMap::new();
    for (i, output_symbol) in func.graph.output_vals.iter().enumerate() {
        output_idxs.insert(output_symbol.clone(), i);
    }

    let mut output_vals: Vec<usize> = vec![0; func.graph.output_vals.len()];

    let mut input_idxs: HashMap<Symbol, usize> = HashMap::new();
    for (i, input_symbol) in func.graph.input_vals.iter().enumerate() {
        input_idxs.insert(input_symbol.clone(), i);
    }

    let mut input_vals: Vec<usize> = vec![0; func.graph.input_vals.len()];

    let mut constant_vals: Vec<usize> = Vec::new();

    // get the function values
    let mut values: Vec<FuncVal> = Vec::with_capacity(func.graph.values.len());

    for (i, val) in func.graph.values.iter().enumerate() {
        let dependents: Vec<usize> = val_deps.get(&val.symbol).cloned().unwrap_or_default();

        let mut constant: Option<PointerLive> = match &val.constant {
            Some(constant_cc_data) => {
                let mut const_path = func_symbol_path.clone();
                const_path.push(val.symbol.clone());
                const_path.push("constant".to_string());
                let constant_stored_data: StoredData = cc_data_to_stored(constant_cc_data, static_state)?;
                let constant_ptr: PointerLive = static_state.buffer(&const_path)?;
                static_state.set(&const_path, constant_stored_data)?;

                Some(constant_ptr)
            },
            None => None
        };

        if let Some(static_val_const_ptr) = static_val_constants.get(&val.symbol) {
            constant = Some(static_val_const_ptr.clone());
        }

        if constant.is_some() {
            constant_vals.push(i);
        }

        let mut arg_for: Vec<(usize, usize)> = Vec::new();
        if let Some(arg_list) = val_as_args.get(&val.symbol) {
            arg_for.extend(arg_list.iter().cloned());
        }

        let func_val = FuncVal {
            symbol: val.symbol.clone(),
            index: i,
            dependents,
            constant,
            output_idx: output_idxs.get(&val.symbol).cloned(),
            arg_for
        };

        values.push(func_val);

        if let Some(input_idx) = input_idxs.get(&val.symbol) {
            input_vals[*input_idx] = i;
        }

        if let Some(output_idx) = output_idxs.get(&val.symbol) {
            output_vals[*output_idx] = i;
        }
    }

    let res = FuncLive {
        symbol_path: func_symbol_path.clone(),
        values,
        ops,
        input_vals,
        output_vals,
        constant_vals,
        call_ops,
    };

    Ok(res)
}

fn resolve_import_path(root_symbol_path: &SymbolPath, import_path: &SymbolPath) -> SymbolPath {
    if import_path.first() == root_symbol_path.first() {
        import_path.clone()
    } else if import_path.first() == root_symbol_path.last() || import_path.first().is_some_and(|segment| segment == "root") {
        let mut resolved = root_symbol_path.clone();
        resolved.extend(import_path.iter().skip(1).cloned());
        resolved
    } else {
        let mut resolved = root_symbol_path.clone();
        resolved.extend(import_path.iter().cloned());
        resolved
    }
}

fn validate_import_cycles(
    collection: &Collection,
    root_symbol_path: &SymbolPath,
    collection_symbol_path: &SymbolPath,
) -> ExecResult<()> {
    let Some(imports) = &collection.imports else {
        return Ok(());
    };

    let mut import_graph: HashMap<&str, String> = HashMap::new();
    for (name, import_path) in imports {
        let resolved_import_path = resolve_import_path(root_symbol_path, import_path);
        if resolved_import_path.len() == collection_symbol_path.len() + 1
            && resolved_import_path.starts_with(collection_symbol_path)
        {
            let target_symbol = resolved_import_path.last().unwrap();
            if imports.contains_key(target_symbol) {
                import_graph.insert(name.as_str(), target_symbol.clone());
            }
        }
    }

    let mut visiting: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();

    fn visit(
        symbol: &str,
        import_graph: &HashMap<&str, String>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        collection_symbol_path: &SymbolPath,
    ) -> ExecResult<()> {
        if visited.contains(symbol) {
            return Ok(());
        }

        if !visiting.insert(symbol.to_string()) {
            let cycle_start = stack.iter().position(|name| name == symbol).unwrap_or(0);
            let mut cycle: Vec<String> = stack[cycle_start..].to_vec();
            cycle.push(symbol.to_string());
            let cycle = cycle.join(" -> ");
            return Err(format!(
                "Import cycle detected in {:?}: {}",
                collection_symbol_path,
                cycle
            ));
        }

        stack.push(symbol.to_string());

        if let Some(next) = import_graph.get(symbol) {
            visit(next, import_graph, visiting, visited, stack, collection_symbol_path)?;
        }

        stack.pop();
        visiting.remove(symbol);
        visited.insert(symbol.to_string());
        Ok(())
    }

    for symbol in imports.keys() {
        visit(
            symbol.as_str(),
            &import_graph,
            &mut visiting,
            &mut visited,
            &mut stack,
            collection_symbol_path,
        )?;
    }

    Ok(())
}

pub fn fill_collection(
    static_state: &mut StaticState,
    root_collection: &Collection,
    root_symbol_path: &SymbolPath,
    symbol_path: &SymbolPath,
    value: &Collection
) -> ExecResult<()> {
    // store the constants at each location
    if let Some(constants) = &value.constants {
        for (name, constant) in constants {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            static_ref.set(collection_const_data_to_stored(constant, static_state)?)
                .map_err(|_| format!("Error setting constant at path {:?}", path))?;
        }
    }

    validate_import_cycles(value, root_symbol_path, symbol_path)?;

    // store the static reference to the imported collections at each import location
    if let Some(imports) = &value.imports {
        for (name, import_path) in imports {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let resolved_import_path = resolve_import_path(root_symbol_path, import_path);
            let import_static_ref: StaticRefLive = static_state.get_ref(&resolved_import_path).map_err(|_| {
                format!(
                    "Import '{}' in {:?} points to missing target {:?}",
                    name,
                    symbol_path,
                    resolved_import_path
                )
            })?;

            static_ref.set(StoredData::StaticRefStored(import_static_ref))
                .map_err(|_| format!("Error setting import at path {:?}", path))?;
        }
    }

    // store the custom types at each location
    if let Some(types) = &value.types {
        for (name, type_def) in types {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            validate_custom_type_fields(name, type_def, symbol_path)?;

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let live_type: TypeLive = type_def_to_live_type(type_def, static_state, name)?;
            static_ref.set(StoredData::TypeStored(live_type))
                .map_err(|_| format!("Error setting type at path {:?}", path))?;
        }
    }

    // store the functions at each location
    if let Some(functions) = &value.functions {
        validate_local_call_cycles(value.functions.as_ref(), symbol_path)?;

        for (name, func) in functions {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            let static_ref: StaticRefLive = static_state.get_ref(&path)?;
            let live_func: FuncLive = cl_func_to_live_func(func, &path, value.functions.as_ref(), value.types.as_ref(), value.imports.as_ref(), root_collection, root_symbol_path, static_state)?;
            static_ref.set(StoredData::FuncStored(live_func))
                .map_err(|_| format!("Error setting function at path {:?}", path))?;
        }
    }

    // fill the sub-collections
    if let Some(collections) = &value.collections {
        for (name, sub_collection) in collections {
            let mut path: SymbolPath = symbol_path.clone();
            path.push(name.clone());

            fill_collection(static_state, root_collection, root_symbol_path, &path, sub_collection)?;
        }
    }

    Ok(())
}

pub fn bind_program(
    program: Collection,
    static_state: &mut StaticState,
    main_symbol: Option<Symbol>
) -> ExecResult<PointerLive> {
    let main_symbol: Symbol = main_symbol.unwrap_or_else(|| "main".to_string());
    let main_path: SymbolPath = vec![main_symbol.clone()];

    validate_collection_symbol_uniqueness(&main_path, &program)?;

    let main_ptr: PointerLive = match buffer_collection(static_state, &main_path, &program) {
        Ok(v) => v,
        Err(e) => return Err(format!("Error buffering main collection: {}", e)),
    };

    match fill_collection(static_state, &program, &main_path, &main_path, &program) {
        Ok(_) => {},
        Err(e) => return Err(format!("Error filling main collection buffers: {}", e)),
    }

    Ok(main_ptr)
}