use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use crate::binder::intermediate::r#const::{CCData, store_cc_data};
use crate::binder::intermediate::r#type::CollectionType;
use crate::binder::intermediate::collection::Collection;
use crate::binder::functions::{FunctionGraph, FunctionValueNode};
use crate::binder::Binder;
use crate::runtime::data::live::{PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::data::stored::StoredData::DictStored;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::mmu::mmu::{execute_store, get_primitive_type, value_ref_from_ptr};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::store_op::StoreOp::CreateBuffer;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::operator::operator::execute_op;
use crate::runtime::vm::operator::ops::Operation;

impl Binder {
    /// Create buffers for each member of the collection and stores a dictionary of the buffers in the symbol table.
    /// If it has any sub-intermediate, it will recursively create buffers for those as well.
    fn create_collection_skeleton(&mut self, value: Collection, symbol: Symbol) -> ExecResult<()> {
        let mut buffers: HashMap<Symbol, ValueReference> = HashMap::new();

        if let Some(functions) = value.functions {
            for (symbol, _func) in functions {
                let func_ref = match execute_store(self.mmu.clone(), CreateBuffer) {
                    Ok(result) => result[0].clone(),
                    Err(err) => return Err(format!("Error creating buffer for function {}: {}", symbol, err))
                };
                buffers.insert(symbol, func_ref);
            }
        }

        if let Some(constants) = value.constants {
            for (symbol, _constant) in constants {
                let constant_ref = match execute_store(self.mmu.clone(), CreateBuffer) {
                    Ok(result) => result[0].clone(),
                    Err(err) => return Err(format!("Error creating buffer for constant {}: {}", symbol, err))
                };
                buffers.insert(symbol, constant_ref);
            }
        }

        if let Some(types) = value.types {
            for (symbol, _type) in types {
                let type_ref = match execute_store(self.mmu.clone(), CreateBuffer) {
                    Ok(result) => result[0].clone(),
                    Err(err) => return Err(format!("Error creating buffer for type {}: {}", symbol, err))
                };
                buffers.insert(symbol, type_ref);
            }
        }

        if let Some(import) = value.imports {
            for (symbol, _import) in import {
                let import_ref = match execute_store(self.mmu.clone(), CreateBuffer) {
                    Ok(result) => result[0].clone(),
                    Err(err) => return Err(format!("Error creating buffer for import {}: {}", symbol, err))
                };
                buffers.insert(symbol, import_ref);
            }
        }

        if let Some(collections) = value.collections {
            for (symbol, _sub_collection) in collections {
                let sub_collection_symbol = format!("{}.{}", symbol, symbol);
                self.create_collection_skeleton(_sub_collection, sub_collection_symbol.clone())?;
                let sub_collection_ref = self.symbol_table.get(&sub_collection_symbol).unwrap().clone();
                buffers.insert(symbol, sub_collection_ref);

                // drop the sub-collection from the symbol table
                self.symbol_table.remove(&sub_collection_symbol);
            }
        }

        // store a dict of the buffers
        let buffers_ref = match execute_store(self.mmu.clone(), StoreOp::StoreDict(buffers.iter().map(|(symbol, val_ref)| (symbol.clone(), val_ref)).collect())) {
            Ok(result) => result[0].clone(),
            Err(err) => return Err(format!("Error creating buffer for collection {}: {}", symbol, err))
        };

        self.symbol_table.insert(symbol, buffers_ref);
        Ok(())
    }

    pub fn store_named_cc_data(&mut self, data: CCData, symbol: Symbol) -> ExecResult<()> {
        let data_refs = store_cc_data(self.mmu.clone(), data)?;
        self.symbol_table.insert(symbol, data_refs[0].clone());
        Ok(())
    }

    pub fn store_multiple_named_cc_data(&mut self, data: Vec<CCData>, prefix: Symbol) -> ExecResult<Vec<Symbol>> {
        let mut symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
        let mut symbols: Vec<Symbol> = Vec::new();

        for (i, data) in data.into_iter().enumerate() {
            let symbol = format!("{}{}", prefix, i);
            let data_refs = store_cc_data(self.mmu.clone(), data)?;
            symbol_table.insert(symbol.clone(), data_refs[0].clone());
            symbols.push(symbol);
        }

        self.symbol_table.extend(symbol_table);
        Ok(symbols)
    }

    fn fill_collection_skeleton(&mut self, collection_ref: ValueReference, value: Collection) -> ExecResult<()> {
        let collection_val: Arc<StoredData> = match self.mmu.get_ref_value(&collection_ref) {
            Ok(collection_val) => collection_val,
            Err(err) => return Err(format!("Error getting collection: {}", err))
        };
        let collection = match collection_val.as_ref() {
            DictStored(dict) => dict,
            _ => return Err("Collection is not a dict.".to_string()),
        };

        // fill the import buffers with pointers to the external values
        if let Some(imports) = value.imports {
            for (symbol, import_path) in imports {
                let import_ref = match self.get_path(import_path) {
                    Ok(import_ref) => import_ref,
                    Err(err) => return Err(format!("Error getting import {} for collection {}: {}", symbol, symbol, err))
                };

                let buffer_ptr = match collection.get(&symbol) {
                    Some(buffer_ptr) => buffer_ptr,
                    None => return Err(format!("Buffer for import {} not found for collection {}.", symbol, symbol))
                };
                let buffer_ref = match value_ref_from_ptr(self.mmu.clone(), buffer_ptr.clone()) {
                    Ok(buffer_ref) => buffer_ref,
                    Err(err) => return Err(format!("Error getting buffer reference for import {} for collection {}: {}", symbol, symbol, err))
                };

                let fill_result = execute_op(self.mmu.clone(), Operation::SetBuffer(&buffer_ref, StoredData::PointerStored(import_ref.pointer.clone())));
                if let Err(err) = fill_result {
                    return Err(format!("Error filling buffer for import {} for collection {}: {}", symbol, symbol, err));
                }
            }
        }

        // fill the constant buffers with the constant values
        if let Some(constants) = value.constants {
            for (symbol, constant) in constants {
                let buffer_ptr = match collection.get(&symbol) {
                    Some(buffer_ptr) => buffer_ptr,
                    None => return Err(format!("Buffer for constant {} not found for collection {}.", symbol, symbol))
                };
                let buffer_ref = match value_ref_from_ptr(self.mmu.clone(), buffer_ptr.clone()) {
                    Ok(buffer_ref) => buffer_ref,
                    Err(err) => return Err(format!("Error getting buffer reference for constant {} for collection {}: {}", symbol, symbol, err))
                };

                // store the constant data in memory in a temporary location
                let stored_ref = store_cc_data(self.mmu.clone(), constant.into())?[0].clone();

                // retrieve the value that was stored
                let stored_value = match self.mmu.get_ref_value(&stored_ref) {
                    Ok(stored_value) => stored_value,
                    Err(err) => return Err(format!("Error getting stored value for constant {} for collection {}: {}", symbol, symbol, err))
                };

                // fill the buffer with the stored data
                let fill_result = execute_op(self.mmu.clone(), Operation::SetBuffer(&buffer_ref, (*stored_value).clone()));
                if let Err(err) = fill_result {
                    return Err(format!("Error filling buffer for constant {} for collection {}: {}", symbol, symbol, err));
                }

                // drop the reference to the temporary location
            }
        }

        // fill the function buffers with the function graphs
        if let Some(functions) = value.functions {
            for (symbol, func) in functions {
                let buffer_ptr = match collection.get(&symbol) {
                    Some(buffer_ptr) => buffer_ptr,
                    None => return Err(format!("Buffer for function {} not found for collection {}.", symbol, symbol))
                };
                let buffer_ref = match value_ref_from_ptr(self.mmu.clone(), buffer_ptr.clone()) {
                    Ok(buffer_ref) => buffer_ref,
                    Err(err) => return Err(format!("Error getting buffer reference for function {} for collection {}: {}", symbol, symbol, err))
                };

                // convert the function graph to a proper format by storing the constant values in memory
                let mut func_constants: Vec<ValueReference> = vec![];
                let mut func_graph = FunctionGraph {
                    values: vec![],
                    ops: func.graph.ops,
                    input_vals: func.graph.input_vals,
                    output_vals: func.graph.output_vals,
                };

                for c_func_value_node in func.graph.values {
                    let graph;
                    if let Some(constant) = c_func_value_node.constant {
                        let stored_ref = store_cc_data(self.mmu.clone(), constant.clone())?[0].clone();
                        let stored_value: Arc<StoredData> = match self.mmu.get_ref_value(&stored_ref) {
                            Ok(stored_value) => stored_value,
                            Err(err) => return Err(format!("Error getting stored value for constant {} for function {} for collection {}: {}", c_func_value_node.symbol, symbol, symbol, err))
                        };
                        graph = FunctionValueNode {
                            symbol: c_func_value_node.symbol,
                            constant: Some((*stored_value).clone()),
                        };
                        // temporarily hold on to the ref to prevent child pointers from being dropped
                        func_constants.push(stored_ref);
                    } else {
                        graph = FunctionValueNode {
                            symbol: c_func_value_node.symbol,
                            constant: None,
                        };
                    }
                    func_graph.values.push(graph);
                }

                // temporarily store the function graph in memory at a random location
                let func_ref = match execute_store(self.mmu.clone(), StoreOp::StoreFunctionGraph(func_graph, Some(&collection_ref))) {
                    Ok(result) => result[0].clone(),
                    Err(err) => return Err(format!("Error storing function {} for collection {}: {}", symbol, symbol, err))
                };

                // should now be ok to drop the constant refs
                drop(func_constants);

                // get the stored data for the function graph
                let func_stored = match self.mmu.get_ref_value(&func_ref) {
                    Ok(func_stored) => func_stored,
                    Err(err) => return Err(format!("Error getting stored data for function {} for collection {}: {}", symbol, symbol, err))
                };

                // fill the buffer with the stored data
                let fill_result = execute_op(self.mmu.clone(), Operation::SetBuffer(&buffer_ref, (*func_stored).clone()));
                if let Err(err) = fill_result {
                    return Err(format!("Error filling buffer for function {} for collection {}: {}", symbol, symbol, err));
                }
            }
        }

        // fill the type buffers with the type values
        if let Some(types) = value.types {
            for (symbol, type_def) in types {
                let buffer_ptr = match collection.get(&symbol) {
                    Some(buffer_ptr) => buffer_ptr,
                    None => return Err(format!("Buffer for type {} not found for collection {}.", symbol, symbol))
                };
                let buffer_ref = match value_ref_from_ptr(self.mmu.clone(), buffer_ptr.clone()) {
                    Ok(buffer_ref) => buffer_ref,
                    Err(err) => return Err(format!("Error getting buffer reference for type {} for collection {}: {}", symbol, symbol, err))
                };

                let mut fields: Vec<(Symbol, PointerLive)> = vec![];
                for (field_symbol, field_type_const) in type_def.0 {
                    // get the type of the field
                    let field_type_ref: ValueReference = match field_type_const.0 {
                        CollectionType::Any => get_primitive_type(self.mmu.clone(), &TypeLive::Dynamic).unwrap(),
                        CollectionType::Null => get_primitive_type(self.mmu.clone(), &TypeLive::Null).unwrap(),
                        CollectionType::Int => get_primitive_type(self.mmu.clone() ,&TypeLive::Integer).unwrap(),
                        CollectionType::Float => get_primitive_type(self.mmu.clone(), &TypeLive::Float).unwrap(),
                        CollectionType::Str => get_primitive_type(self.mmu.clone(), &TypeLive::String).unwrap(),
                        CollectionType::Bool => get_primitive_type(self.mmu.clone(), &TypeLive::Boolean).unwrap(),
                        CollectionType::List => get_primitive_type(self.mmu.clone(), &TypeLive::List).unwrap(),
                        CollectionType::Dict => get_primitive_type(self.mmu.clone(), &TypeLive::Dictionary).unwrap(),
                        CollectionType::Type => get_primitive_type(self.mmu.clone(), &TypeLive::Type).unwrap(),
                        CollectionType::Custom(type_symbol) => {
                            match self.symbol_table.get(&type_symbol) {
                                Some(type_ref) => type_ref.clone(),
                                None => return Err(format!("Type {} not found for field {} for type {} for collection {}.", type_symbol, field_symbol, symbol, symbol))
                            }
                        }
                    };

                    fields.push((field_symbol, field_type_ref.pointer.clone()));
                }

                let type_stored: StoredData = StoredData::TypeStored(TypeLive::Custom(symbol.clone(), Uuid::new_v4().to_string(), fields));
                let fill_result = execute_op(self.mmu.clone(), Operation::SetBuffer(&buffer_ref, type_stored));
                if let Err(err) = fill_result {
                    return Err(format!("Error filling buffer for type {} for collection {}: {}", symbol, symbol, err));
                }
            }
        }

        // fill the sub-collection buffers with the sub-intermediate
        if let Some(collections) = value.collections {
            for (symbol, sub_collection) in collections {
                let buffer_ptr = match collection.get(&symbol) {
                    Some(buffer_ptr) => buffer_ptr,
                    None => return Err(format!("Buffer for sub-collection {} not found for collection {}.", symbol, symbol))
                };

                let sub_collection_ref = match value_ref_from_ptr(self.mmu.clone(), buffer_ptr.clone()) {
                    Ok(sub_collection_ref) => sub_collection_ref,
                    Err(err) => return Err(format!("Error getting buffer reference for sub-collection {} for collection {}: {}", symbol, symbol, err))
                };

                self.fill_collection_skeleton(sub_collection_ref, sub_collection)?;
            }
        }

        Ok(())
    }

    pub fn store_collection(&mut self, value: Collection, symbol: Symbol) -> ExecResult<()> {
        self.create_collection_skeleton(value.clone(), symbol.clone())?;

        let collection_ref = self.symbol_table.get(&symbol).unwrap().clone();

        self.fill_collection_skeleton(collection_ref, value)?;

        Ok(())
    }

    pub fn store_collections(&mut self, values: Vec<(Collection, Symbol)>) -> ExecResult<()> {
        // create all of the collection skeletons
        for (value, symbol) in &values {
            self.create_collection_skeleton(value.clone(), symbol.clone())?;
        }

        // fill all of the collection skeletons
        for (value, symbol) in values {
            let collection_ref = self.symbol_table.get(&symbol).unwrap().clone();
            self.fill_collection_skeleton(collection_ref, value)?;
        }

        Ok(())
    }
}


// #[cfg(test)]
// mod tests {
//     use std::intermediate::HashMap;
//     use maplit::hashmap;
//     use crate::binder::intermediate::c_const::CCData;
//     use crate::binder::intermediate::collection::Collection;
//     use crate::binder::intermediate::func::{CFnValueNode, CollectionFunc, CollectionFuncGraph};
//     use crate::binder::functions::FunctionOpNode;
//     use crate::binder::GraphiteApi;
//     use crate::binder::interface::VmInterface;
//     use crate::runtime::data::live::LiveData;
//     use crate::runtime::data::functions::OpCode;
//     use crate::runtime::vm::VM;
//
//     #[test]
//     fn test_store_coded_collection() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let mut binder = GraphiteApi { vm, symbol_table: HashMap::new() };
//
//             let my_list = vec![10, 20, 30];
//
//             let collection = Collection {
//                 constants: Some(hashmap! {
//                     "two".into() => 2.into(),
//                     "my_list".into() => my_list.iter().map(|v| v.clone().into()).collect::<Vec<CCData>>().into(),
//                     "my_dict".into() => hashmap!{
//                         "Hello".to_string() => "World".to_string().into(),
//                         "Foo".to_string() => "Bar".to_string().into()
//                     }.into(),
//                 }),
//                 functions: Some(hashmap! {
//                     "double".into() => CollectionFunc {graph: CollectionFuncGraph {
//                         values: vec![
//                             CFnValueNode::constant("_two".into(), CCData::String("two".to_string())),
//                             CFnValueNode::var("two".into()),
//
//                             CFnValueNode::var("num".into()),
//                             CFnValueNode::var("doubled".into()),
//                         ],
//                         ops: vec![
//                             // get the 2 const
//                             FunctionOpNode::new(OpCode::Get, vec!["outer".into(), "_two".into()], "two".into()),
//
//                             // double the number
//                             FunctionOpNode::new(OpCode::Mul, vec!["num".into(), "two".into()], "doubled".into())
//                         ],
//                         input_vals: vec!["num".into()],
//                         output_vals: vec!["doubled".into()],
//                     }},
//                     "double_list".into() => CollectionFunc {graph: CollectionFuncGraph {
//                         values: vec![
//                             CFnValueNode::var("double_func".into()),
//                             CFnValueNode::constant("_double".into(), CCData::String("double".to_string())),
//                             CFnValueNode::constant("_my_list".into(), CCData::String("my_list".to_string())),
//
//                             CFnValueNode::var("my_list".into()),
//                             CFnValueNode::var("double_list".into()),
//                         ],
//                         ops: vec![
//                             // get the double func and the list
//                             FunctionOpNode::new(OpCode::Get, vec!["outer".into(), "_double".into()], "double_func".into()),
//                             FunctionOpNode::new(OpCode::Get, vec!["outer".into(), "_my_list".into()], "my_list".into()),
//
//                             FunctionOpNode::new(OpCode::Map, vec!["double_func".into(), "my_list".into()], "double_list".into())
//                         ],
//                         input_vals: vec![],
//                         output_vals: vec!["double_list".into()],
//                     }},
//                 }),
//                 intermediate: None,
//                 imports: None,
//                 types: None,
//             };
//
//             binder.store_collection(collection, "my_collection".to_string()).unwrap();
//
//             binder.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();
//
//             let result = binder.get("doubled_list".to_string()).unwrap();
//             let result = result.as_live().as_list().unwrap().unwrap();
//
//             assert_eq!(result.len(), 3);
//
//             for i in 0..result.len() {
//                 let item = result.get(i).unwrap();
//                 let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
//                 let item = vm.get_ref_value(&item_ref).unwrap();
//                 let item = item.as_live().as_int().unwrap().unwrap();
//
//                 assert_eq!(item, my_list[i] * 2);
//             }
//         }
//
//         // there should be 0 objects in the VM
//         assert_eq!(vm.object_count(), 0);
//
//     }
//
//     #[test]
//     fn test_deserialize_store_collection() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let mut binder = GraphiteApi { vm, symbol_table: HashMap::new() };
//
//             let json_collection = r#"{
//                 "constants": {
//                     "two": 2,
//                     "my_list": [10, 20, 30],
//                     "my_dict": {
//                         "Hello": "World",
//                         "Foo": "Bar"
//                     }
//                 },
//                 "functions": {
//                     "double": {
//                        "name": "Double",
//                        "description": "Doubles a number",
//                        "graph": {
//                             "values": [
//                                 ["_two", "two"],
//                                 "two",
//                                 "num",
//                                 "doubled"
//                             ],
//                             "ops": [
//                                 ["Get", ["outer", "_two"], "two"],
//                                 ["Mul", ["num", "two"], "doubled"]
//                             ],
//                             "input_vals": ["num"],
//                             "output_vals": ["doubled"]
//                         }
//                     },
//                     "double_list": {
//                         "name": "Double List",
//                         "description": "Doubles a list of numbers",
//                         "graph": {
//                             "values": [
//                                 "double_func",
//                                 ["_double", "double"],
//                                 ["_my_list", "my_list"],
//                                 "my_list",
//                                 "double_list"
//                             ],
//                             "ops": [
//                                 ["Get", ["outer", "_double"], "double_func"],
//                                 ["Get", ["outer", "_my_list"], "my_list"],
//                                 ["Map", ["double_func", "my_list"], "double_list"]
//                             ],
//                             "input_vals": [],
//                             "output_vals": ["double_list"]
//                         }
//                     }
//                 },
//                 "intermediate": {},
//                 "imports": {},
//                 "types": {}
//             }"#;
//
//             let collection: Collection = serde_json::from_str(json_collection).unwrap();
//
//             binder.store_collection(collection, "my_collection".to_string()).unwrap();
//
//             binder.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();
//
//             let result = binder.get("doubled_list".to_string()).unwrap();
//             let result = result.as_live().as_list().unwrap().unwrap();
//
//             assert_eq!(result.len(), 3);
//             let my_list = vec![10, 20, 30];
//
//             for i in 0..result.len() {
//                 let item = result.get(i).unwrap();
//                 let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
//                 let item = vm.get_ref_value(&item_ref).unwrap();
//                 let item = item.as_live().as_int().unwrap().unwrap();
//
//                 assert_eq!(item, my_list[i] * 2);
//             }
//         }
//
//         // there should be 0 objects in the VM
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_test_deserialize_store_collection_literal_list() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//
//         {
//             let mut binder = GraphiteApi { vm, symbol_table: HashMap::new() };
//
//             let json_collection = r#"{
//                 "constants": {
//                     "two": 2
//                 },
//                 "functions": {
//                     "double": {
//                        "name": "Double",
//                        "description": "Doubles a number",
//                        "graph": {
//                             "values": [
//                                 ["_two", "two"],
//                                 "two",
//                                 "num",
//                                 "doubled"
//                             ],
//                             "ops": [
//                                 ["Get", ["outer", "_two"], "two"],
//                                 ["Mul", ["num", "two"], "doubled"]
//                             ],
//                             "input_vals": ["num"],
//                             "output_vals": ["doubled"]
//                         }
//                     },
//                     "double_list": {
//                         "name": "Double List",
//                         "description": "Doubles a list of numbers",
//                         "graph": {
//                             "values": [
//                                 "double_func",
//                                 ["_double", "double"],
//                                 ["my_list", [1, 2, 3]],
//                                 "double_list",
//                                 ["null", null]
//                             ],
//                             "ops": [
//                                 ["Get", ["outer", "_double"], "double_func"],
//                                 ["Map", ["double_func", "my_list"], "double_list"]
//                             ],
//                             "input_vals": [],
//                             "output_vals": ["double_list"]
//                         }
//                     }
//                 },
//                 "intermediate": {},
//                 "imports": {},
//                 "types": {}
//             }"#;
//
//
//
//             let collection: Collection = match serde_json::from_str(json_collection) {
//                 Ok(collection) => collection,
//                 Err(e) => {
//                     println!("{}", e);
//                     panic!();
//                 }
//             };
//
//             binder.store_collection(collection, "my_collection".to_string()).unwrap();
//             binder.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();
//
//             let result = binder.get("doubled_list".to_string()).unwrap();
//             let result = result.as_live().as_list().unwrap().unwrap();
//
//             assert_eq!(result.len(), 3);
//             let my_list = vec![1, 2, 3];
//
//             for i in 0..result.len() {
//                 let item = result.get(i).unwrap();
//                 let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
//                 let item = vm.get_ref_value(&item_ref).unwrap();
//                 let item = item.as_live().as_int().unwrap().unwrap();
//
//                 assert_eq!(item, my_list[i] * 2);
//             }
//         }
//
//         // there should be 0 objects in the VM
//         assert_eq!(vm.object_count(), 0);
//     }
//
//     #[test]
//     fn test_store_collection_with_types<'a>() {
//         let vm: &mut VM = &mut VM::new(2, 2);
//         {
//
//             let mut binder = GraphiteApi { vm, symbol_table: HashMap::new() };
//
//             let json_collection = r#"{
//                 "types": {
//                     "Person": [
//                         ["name", "str"],
//                         ["age", "int"]
//                     ]
//                 },
//                 "functions": {
//                     "main": {
//                         "graph": {
//                             "input_vals": [],
//                             "output_vals": ["doubled_age"],
//                             "values": [
//                                 ["two", 2],
//                                 ["name", "John"],
//                                 ["_age", "age"],
//                                 ["age", 30],
//                                 ["_Person", "Person"],
//                                 "Person",
//                                 "john",
//                                 "john_age",
//                                 "doubled_age"
//                             ],
//                             "ops": [
//                                 ["Get", ["outer", "_Person"], "Person"],
//                                 ["Init", ["Person", "name", "age"], "john"],
//                                 ["Get", ["john", "_age"], "john_age"],
//                                 ["Mul", ["john_age", "two"], "doubled_age"]
//                             ]
//                         }
//                     }
//                 }
//             }"#;
//
//             let collection: Collection = match serde_json::from_str(json_collection) {
//                 Ok(collection) => collection,
//                 Err(e) => {
//                     println!("{}", e);
//                     panic!();
//                 }
//             };
//
//             binder.store_collection(collection, "my_collection".to_string()).unwrap();
//
//             binder.execute(vec!["my_collection".to_string(), "main".to_string()], vec![], vec!["doubled_age".to_string()]).unwrap();
//
//             let result = binder.get("doubled_age".to_string()).unwrap();
//
//             let result = result.as_live().as_int().unwrap().unwrap();
//
//             assert_eq!(result, 60);
//         }
//     }
// }