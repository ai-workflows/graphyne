use std::collections::HashMap;
use crate::api::collections::c_const::CCData;
use crate::api::collections::collection::Collection;
use crate::api::functions::{FunctionGraph, FunctionValueNode};
use crate::api::interface::{VmInterface};
use crate::core::data::live::{LiveData, IntLive, FloatLive, StringLive, BoolLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult, Symbol, SymbolPath};
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::stored::StoredData::DictStored;
use crate::core::vm::ops::Operation;
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::store_op::StoreOp::{CreateBuffer, StoreBool, StoreFloat, StoreInt, StoreString};
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;


pub struct GraphiteApi<'a> {
    pub vm: &'a VM,
    pub symbol_table: HashMap<Symbol, ValueReference<'a>>,
}

impl<'a> GraphiteApi<'a> {
    fn store_value(&mut self, operation: StoreOp, symbol: Symbol) -> ExecResult<()> {
        let store_result: Vec<ValueReference> = self.vm.execute_store(operation).unwrap();
        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    /// Gets the reference to a value at the given symbol path.
    pub fn get_path(&self, path: SymbolPath) -> ExecResult<ValueReference> {
        // represent the symbol table as a stored dict
        let mut context: Option<ValueReference> = None;

        for symbol in path.clone() {
            // get the value of the current context, or convert the symbol table to a dict if this is the first iteration
            let context_value: StoredData = match context {
                Some(context) => match self.vm.get_ref_value(&context) {
                    Ok(context_stored) => context_stored,
                    Err(err) => return Err(format!("Error getting symbol {} for path {:?}: {}", symbol, path.clone(), err))
                },
                None => {
                    DictStored(self.symbol_table.iter().map(|(symbol, val_ref)| (symbol.clone(), val_ref.pointer.clone())).collect::<HashMap<Symbol, PointerLive>>())
                }
            };

            // get the current context as a dict
            let dict = match context_value {
                DictStored(dict) => dict,
                _ => return Err(format!("Context at symbol {} for path {:?} is not a dict.", symbol, path))
            };

            // get the pointer at the current symbol
            let symbol_ptr = match dict.get(&symbol) {
                Some(symbol_ptr) => symbol_ptr,
                None => return Err(format!("Symbol {} not found for path {:?}.", symbol, path))
            };

            // convert the pointer to a value reference
            context = match self.vm.value_ref_from_ptr(symbol_ptr.clone()) {
                Ok(symbol_ref) => Some(symbol_ref),
                Err(err) => return Err(format!("Error getting value reference for symbol {} for path {:?}: {}", symbol, path, err))
            };
        }

        Ok(context.unwrap())
    }

    /// Create buffers for each member of the collection and stores a dictionary of the buffers in the symbol table.
    /// If it has any sub-collections, it will recursively create buffers for those as well.
    fn create_collection_skeleton(&mut self, value: Collection, symbol: Symbol) -> ExecResult<()> {
        let mut buffers: HashMap<Symbol, ValueReference> = HashMap::new();

        for (symbol, _func) in value.functions {
            let func_ref = match self.vm.execute_store(CreateBuffer) {
                Ok(result) => result[0].clone(),
                Err(err) => return Err(format!("Error creating buffer for function {}: {}", symbol, err))
            };
            buffers.insert(symbol, func_ref);
        }

        for (symbol, _constant) in value.constants {
            let constant_ref = match self.vm.execute_store(CreateBuffer) {
                Ok(result) => result[0].clone(),
                Err(err) => return Err(format!("Error creating buffer for constant {}: {}", symbol, err))
            };
            buffers.insert(symbol, constant_ref);
        }

        for (symbol, _import) in value.imports {
            let import_ref = match self.vm.execute_store(CreateBuffer) {
                Ok(result) => result[0].clone(),
                Err(err) => return Err(format!("Error creating buffer for import {}: {}", symbol, err))
            };
            buffers.insert(symbol, import_ref);
        }

        for (sub_symbol, sub_collection) in value.collections {
            let sub_collection_symbol = format!("{}.{}", symbol, sub_symbol);

            self.create_collection_skeleton(sub_collection, sub_collection_symbol.clone())?;
            let sub_collection_ref = self.symbol_table.get(&sub_collection_symbol).unwrap().clone();
            buffers.insert(sub_symbol, sub_collection_ref);

            // drop the sub-collection from the symbol table
            self.symbol_table.remove(&sub_collection_symbol);
        }

        // store a dict of the buffers
        let buffers_ref = match self.vm.execute_store(StoreOp::StoreDict(buffers.iter().map(|(symbol, val_ref)| (symbol.clone(), val_ref)).collect())) {
            Ok(result) => result[0].clone(),
            Err(err) => return Err(format!("Error creating buffer for collection {}: {}", symbol, err))
        };

        self.symbol_table.insert(symbol, buffers_ref);
        Ok(())
    }

    // pub fn store_cc_data(&self, data: CCData) -> ExecResult<Vec<ValueReference>> {
    //     match data {
    //         CCData::Int(i) => self.vm.execute_store(StoreInt(i)),
    //         CCData::Float(f) => self.vm.execute_store(StoreFloat(f)),
    //         CCData::String(s) => self.vm.execute_store(StoreString(s)),
    //         CCData::Bool(b) => self.vm.execute_store(StoreBool(b)),
    //         CCData::List(l) => {
    //             let item_refs: Vec<Vec<ValueReference>> = l.iter().map(|c| self.store_cc_data(c.clone()).unwrap()).collect::<Vec<Vec<ValueReference>>>();
    //             let item_refs: Vec<ValueReference> = item_refs.into_iter().flatten().collect();
    //
    //             self.vm.execute_store(StoreOp::StoreList(item_refs.iter().collect()))
    //         }
    //         CCData::Dict(d) => {
    //             let item_refs: Vec<(String, Vec<ValueReference>)> = d.iter().map(|(k, v)| (k.clone(), self.store_cc_data(v.clone()).unwrap())).collect::<Vec<(String, Vec<ValueReference>)>>();
    //             let item_refs: HashMap<String, ValueReference> = item_refs.into_iter().map(|(k, v)| (k, v[0].clone())).collect();
    //             let item_refs: HashMap<String, &ValueReference> = item_refs.iter().map(|(k, v)| (k.clone(), v)).collect();
    //
    //             self.vm.execute_store(StoreOp::StoreDict(item_refs))
    //
    //         }
    //     }
    // }

    pub fn store_named_cc_data(&'a mut self, data: CCData, symbol: Symbol) -> ExecResult<()> {
        let data_refs = self.vm.store_cc_data(data)?;
        self.symbol_table.insert(symbol, data_refs[0].clone());
        Ok(())
    }

    pub fn store_multiple_named_cc_data(&'a mut self, data: Vec<CCData>, prefix: Symbol) -> ExecResult<Vec<Symbol>> {
        let mut symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
        let mut symbols: Vec<Symbol> = Vec::new();

        for (i, data) in data.into_iter().enumerate() {
            let symbol = format!("{}{}", prefix, i);
            let data_refs = self.vm.store_cc_data(data)?;
            symbol_table.insert(symbol.clone(), data_refs[0].clone());
            symbols.push(symbol);
        }

        self.symbol_table.extend(symbol_table);
        Ok(symbols)
    }

    fn fill_collection_skeleton(&mut self, collection_ref: ValueReference, value: Collection) -> ExecResult<()> {
        let collection_val = match self.vm.get_ref_value(&collection_ref) {
            Ok(collection_val) => collection_val,
            Err(err) => return Err(format!("Error getting collection: {}", err))
        };
        let collection = match collection_val {
            DictStored(dict) => dict,
            _ => return Err("Collection is not a dict.".to_string()),
        };

        // fill the import buffers with pointers to the external values
        for (symbol, import_path) in value.imports {
            let import_ref = match self.get_path(import_path) {
                Ok(import_ref) => import_ref,
                Err(err) => return Err(format!("Error getting import {} for collection {}: {}", symbol, symbol, err))
            };

            let buffer_ptr = match collection.get(&symbol) {
                Some(buffer_ptr) => buffer_ptr,
                None => return Err(format!("Buffer for import {} not found for collection {}.", symbol, symbol))
            };
            let buffer_ref = match self.vm.value_ref_from_ptr(buffer_ptr.clone()) {
                Ok(buffer_ref) => buffer_ref,
                Err(err) => return Err(format!("Error getting buffer reference for import {} for collection {}: {}", symbol, symbol, err))
            };

            let fill_result = self.vm.execute_op(Operation::SetBuffer(&buffer_ref, StoredData::PointerStored(import_ref.pointer.clone())));
            if let Err(err) = fill_result {
                return Err(format!("Error filling buffer for import {} for collection {}: {}", symbol, symbol, err));
            }
        }

        // fill the constant buffers with the constant values
        for (symbol, constant) in value.constants {
            let buffer_ptr = match collection.get(&symbol) {
                Some(buffer_ptr) => buffer_ptr,
                None => return Err(format!("Buffer for constant {} not found for collection {}.", symbol, symbol))
            };
            let buffer_ref = match self.vm.value_ref_from_ptr(buffer_ptr.clone()) {
                Ok(buffer_ref) => buffer_ref,
                Err(err) => return Err(format!("Error getting buffer reference for constant {} for collection {}: {}", symbol, symbol, err))
            };

            // store the constant data in memory in a temporary location
            let stored_ref = self.vm.store_cc_data(constant.into())?[0].clone();

            // retrieve the value that was stored
            let stored_value = match self.vm.get_ref_value(&stored_ref) {
                Ok(stored_value) => stored_value,
                Err(err) => return Err(format!("Error getting stored value for constant {} for collection {}: {}", symbol, symbol, err))
            };

            // fill the buffer with the stored data
            let fill_result = self.vm.execute_op(Operation::SetBuffer(&buffer_ref, stored_value));
            if let Err(err) = fill_result {
                return Err(format!("Error filling buffer for constant {} for collection {}: {}", symbol, symbol, err));
            }

            // drop the reference to the temporary location
        }

        // fill the function buffers with the function graphs
        for (symbol, func) in value.functions {
            let buffer_ptr = match collection.get(&symbol) {
                Some(buffer_ptr) => buffer_ptr,
                None => return Err(format!("Buffer for function {} not found for collection {}.", symbol, symbol))
            };
            let buffer_ref = match self.vm.value_ref_from_ptr(buffer_ptr.clone()) {
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
                    let stored_ref = self.vm.store_cc_data(constant.clone())?[0].clone();
                    let stored_value = match self.vm.get_ref_value(&stored_ref) {
                        Ok(stored_value) => stored_value,
                        Err(err) => return Err(format!("Error getting stored value for constant {} for function {} for collection {}: {}", c_func_value_node.symbol, symbol, symbol, err))
                    };
                    graph = FunctionValueNode {
                        symbol: c_func_value_node.symbol,
                        constant: Some(stored_value),
                    };
                    // temporarily hold on to the the ref to prevent child pointers from being dropped
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
            let func_ref = match self.vm.execute_store(StoreOp::StoreFunctionGraph(func_graph, Some(&collection_ref))) {
                Ok(result) => result[0].clone(),
                Err(err) => return Err(format!("Error storing function {} for collection {}: {}", symbol, symbol, err))
            };

            // should now be ok to drop the constant refs
            drop(func_constants);

            // get the stored data for the function graph
            let func_stored = match self.vm.get_ref_value(&func_ref) {
                Ok(func_stored) => func_stored,
                Err(err) => return Err(format!("Error getting stored data for function {} for collection {}: {}", symbol, symbol, err))
            };

            // fill the buffer with the stored data
            let fill_result = self.vm.execute_op(Operation::SetBuffer(&buffer_ref, func_stored));
            if let Err(err) = fill_result {
                return Err(format!("Error filling buffer for function {} for collection {}: {}", symbol, symbol, err));
            }
        }

        // fill the sub-collection buffers with the sub-collections
        for (symbol, sub_collection) in value.collections {
            let buffer_ptr = match collection.get(&symbol) {
                Some(buffer_ptr) => buffer_ptr,
                None => return Err(format!("Buffer for sub-collection {} not found for collection {}.", symbol, symbol))
            };

            let sub_collection_ref = match self.vm.value_ref_from_ptr(buffer_ptr.clone()) {
                Ok(sub_collection_ref) => sub_collection_ref,
                Err(err) => return Err(format!("Error getting buffer reference for sub-collection {} for collection {}: {}", symbol, symbol, err))
            };

            self.fill_collection_skeleton(sub_collection_ref, sub_collection)?;
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

    pub fn jsonify(&self, val: &StoredData) -> String {
        match val {
            StoredData::NullStored => "null".to_string(),
            StoredData::IntStored(val) => val.to_string(),
            StoredData::FloatStored(val) => val.to_string(),
            StoredData::StringStored(val) => val.clone(),
            StoredData::BoolStored(val) => val.to_string(),
            StoredData::PointerStored(ptr) => {
                let val_ref = match self.vm.value_ref_from_ptr(ptr.clone()) {
                    Ok(val_ref) => val_ref,
                    Err(_) => return "null".to_string(),
                };
                match self.vm.get_ref_value(&val_ref) {
                    Ok(val) => self.jsonify(&val),
                    Err(_) => return "null".to_string(),
                }
            }
            StoredData::ListStored(list) => {
                let mut result = "[".to_string();
                for (i, item) in list.iter().enumerate() {
                    let ptr_stored = StoredData::PointerStored(item.clone());

                    result.push_str(&self.jsonify(&ptr_stored));

                    if i < list.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push_str("]");
                result
            }
            DictStored(dict) => {
                let mut map = HashMap::new();

                for (key, val) in dict {
                    let ptr_stored = StoredData::PointerStored(val.clone());
                    map.insert(key.clone(), self.jsonify(&ptr_stored));
                }

                serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
            }
            StoredData::FuncStored(val) => {
                let mut map = HashMap::new();

                map.insert("input_vals".to_string(), self.jsonify(&StoredData::ListStored(val.input_vals.clone())));
                map.insert("output_vals".to_string(), self.jsonify(&StoredData::ListStored(val.output_vals.clone())));

                serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
            }
            StoredData::FuncValStored(val) => {
                let mut map = HashMap::new();

                map.insert("guid".to_string(), val.guid.clone());
                map.insert("dependents".to_string(), self.jsonify(&StoredData::ListStored(val.dependents.clone())));
                if let Some(constant) = &val.constant {
                    map.insert("constant".to_string(), self.jsonify(&StoredData::PointerStored(constant.clone())));
                }
                map.insert("is_self".to_string(), self.jsonify(&StoredData::BoolStored(val.is_self)));

                serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
            }
            StoredData::FuncOpStored(val) => {
                let mut map = HashMap::new();

                map.insert("guid".to_string(), val.guid.clone());
                map.insert("opcode".to_string(), self.jsonify(&StoredData::IntStored(val.opcode as i64)));
                map.insert("input_vals".to_string(), self.jsonify(&StoredData::ListStored(val.input_vals.clone())));
                map.insert("output_vals".to_string(), self.jsonify(&StoredData::ListStored(val.output_vals.clone())));

                serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
            }
            StoredData::TypeStored(val) => {
                return match val {
                    TypeLive::Custom(name, guid, fields) => {
                        let mut map = HashMap::new();

                        map.insert("name".to_string(), self.jsonify(&StoredData::StringStored(name.clone())));
                        map.insert("guid".to_string(), self.jsonify(&StoredData::StringStored(guid.clone())));

                        let mut fields_map: HashMap<String, PointerLive> = HashMap::new();

                        for (field_name, field_type_ptr) in fields {
                            fields_map.insert(field_name.clone(), field_type_ptr.clone());
                        }

                        map.insert("fields".to_string(), self.jsonify(&StoredData::DictStored(fields_map)));

                        serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
                    },
                    _ => val.get_name()
                };
            },
            StoredData::ObjectStored(val) => {
                let mut map = HashMap::new();

                map.insert("type".to_string(), self.jsonify(&StoredData::PointerStored(val.type_ptr.clone())));
                map.insert("data".to_string(), self.jsonify(&StoredData::DictStored(val.fields.clone())));

                serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())}
        }
    }
}

impl<'a> VmInterface for GraphiteApi<'a> {
    fn store_int(&mut self, value: IntLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreInt(value), symbol)
    }

    fn store_float(&mut self, value: FloatLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreFloat(value), symbol)
    }

    fn store_string(&mut self, value: StringLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreString(value), symbol)
    }

    fn store_bool(&mut self, value: BoolLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreBool(value), symbol)
    }

    fn store_list(&mut self, values: Vec<Symbol>, symbol: Symbol) -> ExecResult<()> {
        let value_refs: Vec<&ValueReference> = values.into_iter()
            .map(|symbol| self.symbol_table.get(&symbol).unwrap())
            .collect();

        let store_op = StoreOp::StoreList(value_refs);
        let store_result: Vec<ValueReference> = self.vm.execute_store(store_op).unwrap();
        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    fn store_dict(&mut self, values: HashMap<String, Symbol>, symbol: Symbol) -> ExecResult<()> {
        let mut value_refs: HashMap<String, &ValueReference> = HashMap::new();

        for (key, value) in values {
            let value_ref = self.symbol_table.get(&value).unwrap();
            value_refs.insert(key, value_ref);
        }

        let store_op = StoreOp::StoreDict(value_refs);
        let store_result: Vec<ValueReference> = self.vm.execute_store(store_op).unwrap();
        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    fn store_function(&mut self, func: FunctionGraph, symbol: Symbol, class_context: Option<SymbolPath>) -> ExecResult<()> {
        // retrieve the class context if it exists
        let context_val: Option<ValueReference> = match class_context {
            Some(path) => Some(self.get_path(path)?),
            None => None,
        };

        let store_op = StoreOp::StoreFunctionGraph(func, context_val.as_ref());
        let store_result: Vec<ValueReference> = self.vm.execute_store(store_op).unwrap();

        drop(context_val);

        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    fn store_multiple(&mut self, values: Vec<StoreOp>, prefix: Symbol) -> ExecResult<Vec<Symbol>> {
        let mut symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
        let mut symbols: Vec<Symbol> = Vec::new();

        for (i, value) in values.into_iter().enumerate() {
            let symbol = format!("{}{}", prefix, i);
            let store_result: Vec<ValueReference> = self.vm.execute_store(value).unwrap();
            symbol_table.insert(symbol.clone(), store_result[0].clone());
            symbols.push(symbol);
        }

        self.symbol_table.extend(symbol_table);
        Ok(symbols)
    }

    fn get(&self, symbol: Symbol) -> ExecResult<StoredData> {
        let val_ref = self.symbol_table.get(&symbol);

        let val_ref = match val_ref {
            Some(val_ref) => val_ref,
            None => return Err(format!("Symbol {} not found.", symbol)),
        };

        self.vm.get_ref_value(val_ref).map(|stored| stored)
    }

    fn get_ptr(&self, symbol: Symbol) -> ExecResult<PointerLive> {
        let val_ref = self.symbol_table.get(&symbol);

        let val_ref = match val_ref {
            Some(val_ref) => val_ref,
            None => return Err(format!("Symbol {} not found.", symbol)),
        };

        Ok(val_ref.pointer.clone())
    }

    fn drop(&mut self, symbol: Symbol) -> ExecResult<()> {
        let val_ref = self.symbol_table.get(&symbol);

        match val_ref {
            Some(_) => {},
            None => return Err(format!("Symbol {} not found.", symbol)),
        };

        // remove the symbol from the symbol table
        self.symbol_table.remove(&symbol);

        Ok(())
    }

    fn execute(&mut self, func: SymbolPath, inputs: Vec<Symbol>, outputs: Vec<Symbol>) -> ExecResult<()> {
        let func_ref = self.get_path(func.clone())?;
        let get_func_result: StoredData = self.vm.get_ref_value(&func_ref).unwrap();
        let func_sig = get_func_result.as_live().as_func().unwrap().unwrap();

        // verify that the number of inputs is correct
        if func_sig.input_vals.len() != inputs.len() {
            return Err(format!("Number of inputs does not match number of inputs for function {:?}.", func));
        }

        let mut input_refs: Vec<ValueReference> = Vec::new();

        for input in inputs {
            let input_ref = match self.symbol_table.get(&input) {
                Some(input_ref) => input_ref.clone(),
                None => return Err(format!("Input symbol {} not found.", input)),
            };
            input_refs.push(input_ref);
        }

        let exec_result: Vec<ValueReference> = match self.vm.handle_call_function(&func_sig, &input_refs) {
            Ok(result) => result,
            Err(err) => return Err(format!("Error executing function {:?}: {}", func, err)),
        };

        // verify that the number of outputs is correct
        if exec_result.len() != outputs.len() {
            return Err(format!("Expected {} outputs, but got {}", outputs.len(), exec_result.len()));
        }

        drop(func_ref);

        // store the outputs
        for (i, output) in outputs.iter().enumerate() {
            let output_ref = exec_result[i].clone();
            self.symbol_table.insert(output.clone(), output_ref);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::api::functions::{FunctionGraph, FunctionOpNode, FunctionValueNode};
    use crate::api::GraphiteApi;
    use crate::api::interface::VmInterface;
    use crate::core::data::functions::OpCode;
    use crate::core::data::live::LiveData;
    use crate::core::{Symbol};
    use crate::core::data::stored::StoredData::IntStored;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    #[test]
    fn test_api<'a>() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();

            let mut api = GraphiteApi { vm, symbol_table };

            let values: Vec<FunctionValueNode> = vec![
                FunctionValueNode::var("num1".into()),
                FunctionValueNode::var("num2".into()),
                FunctionValueNode::var("sum".into()),
            ];

            let ops: Vec<FunctionOpNode> = vec![
                FunctionOpNode::new(OpCode::Add, vec!["num1".into(), "num2".into()], "sum".into())
            ];

            let graph: FunctionGraph = FunctionGraph::new(values, ops, vec!["num1".into(), "num2".into()], vec!["sum".into()]);

            api.store_function(graph, "add".to_string(), None).unwrap();

            api.store_int(5, "num1".to_string()).unwrap();
            api.store_float(10.0, "num2".to_string()).unwrap();

            api.execute(vec!["add".to_string()], vec!["num1".to_string(), "num2".to_string()], vec!["sum".to_string()]).unwrap();

            let result = api.get("sum".to_string()).unwrap();
            assert_eq!(result.as_live().as_int().unwrap(), Ok(15));
        }

        assert_eq!(vm.object_count(), 0);
    }

    #[test]
    fn test_calculate_statistics<'a>() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let symbol_table: HashMap<Symbol, ValueReference<'a>> = HashMap::new();
            let mut api = GraphiteApi { vm, symbol_table };

            // Define the values (variables).
            let values: Vec<FunctionValueNode> = vec![
                FunctionValueNode::var("list".into()),
                FunctionValueNode::var("sum1".into()),
                FunctionValueNode::var("sum2".into()),
                FunctionValueNode::var("sum_final".into()),
                FunctionValueNode::var("average".into()),
                FunctionValueNode::var("is_large_sum".into()),
                FunctionValueNode::var("sum_as_float".into()),
                FunctionValueNode::var("list_length".into()),
                FunctionValueNode::constant("100".into(), IntStored(100)),
                FunctionValueNode::constant("0".into(), IntStored(0)),
                FunctionValueNode::constant("1".into(), IntStored(1)),
                FunctionValueNode::constant("2".into(), IntStored(2)),
                FunctionValueNode::constant("3".into(), IntStored(3)),
                FunctionValueNode::var("item1".into()),
                FunctionValueNode::var("item2".into()),
                FunctionValueNode::var("item3".into()),
                FunctionValueNode::var("item4".into()),
            ];

            // Define the operations.
            let ops: Vec<FunctionOpNode> = vec![
                // get the items from the list
                FunctionOpNode::new(OpCode::Get, vec!["list".into(), "0".into()], "item1".into()),
                FunctionOpNode::new(OpCode::Get, vec!["list".into(), "1".into()], "item2".into()),
                FunctionOpNode::new(OpCode::Get, vec!["list".into(), "2".into()], "item3".into()),
                FunctionOpNode::new(OpCode::Get, vec!["list".into(), "3".into()], "item4".into()),

                // compute the sum
                FunctionOpNode::new(OpCode::Add, vec!["item1".into(), "item2".into()], "sum1".into()),
                FunctionOpNode::new(OpCode::Add, vec!["item3".into(), "item4".into()], "sum2".into()),
                FunctionOpNode::new(OpCode::Add, vec!["sum1".into(), "sum2".into()], "sum_final".into()),

                // compute the average
                FunctionOpNode::new(OpCode::Length, vec!["list".into()], "list_length".into()),
                FunctionOpNode::new(OpCode::Div, vec!["sum_final".into(), "list_length".into()], "average".into()),

                // compute whether the sum is large
                FunctionOpNode::new(OpCode::GreaterThan, vec!["sum_final".into(), "100".into()], "is_large_sum".into()),

                // get the sum as a float
                FunctionOpNode::new(OpCode::AsFloat, vec!["sum_final".into()], "sum_as_float".into()),
            ];

            // Create the function graph.
            let graph: FunctionGraph = FunctionGraph::new(
                values,
                ops,
                vec!["list".into()], // Initial Inputs
                vec!["sum_final".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()], // Returned Outputs
            );

            // Store the function.
            api.store_function(graph, "calculate_statistics".to_string(), None).unwrap();

            // Test the function with a sample list.
            api.store_int(10, "num1".to_string()).unwrap();
            api.store_int(20, "num2".to_string()).unwrap();
            api.store_int(30, "num3".to_string()).unwrap();
            api.store_int(40, "num4".to_string()).unwrap();

            api.store_list(vec!["num1".into(), "num2".into(), "num3".into(), "num4".into()], "list".to_string()).unwrap();
            api.execute(vec!["calculate_statistics".to_string()], vec!["list".to_string()], vec!["sum".into(), "average".into(), "is_large_sum".into(), "sum_as_float".into()]).unwrap();

            // Retrieve and assert the results.
            let sum_result = api.get("sum".to_string()).unwrap();
            let average_result = api.get("average".to_string()).unwrap();
            let is_large_sum_result = api.get("is_large_sum".to_string()).unwrap();
            let sum_as_float_result = api.get("sum_as_float".to_string()).unwrap();

            assert_eq!(sum_result.as_live().as_int().unwrap(), Ok(100));
            assert_eq!(average_result.as_live().as_float().unwrap(), Ok(25.0));
            assert_eq!(is_large_sum_result.as_live().as_bool().unwrap(), Ok(false));
            assert_eq!(sum_as_float_result.as_live().as_float().unwrap(), Ok(100.0));
        }

        assert_eq!(vm.object_count(), 0);
    }
}