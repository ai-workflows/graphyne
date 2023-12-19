use std::collections::HashMap;
use crate::api::collections::c_const::CCData;
use crate::api::collections::collection::Collection;
use crate::api::functions::FunctionGraph;
use crate::api::interface::{VmInterface};
use crate::core::data::live::{LiveData, IntLive, FloatLive, StringLive, BoolLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult, Symbol, SymbolPath};
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
    fn get(&self, path: SymbolPath) -> ExecResult<ValueReference> {
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

    fn store_cc_data(&self, data: CCData) -> ExecResult<Vec<ValueReference>> {
        match data {
            CCData::Int(i) => self.vm.execute_store(StoreInt(i)),
            CCData::Float(f) => self.vm.execute_store(StoreFloat(f)),
            CCData::String(s) => self.vm.execute_store(StoreString(s)),
            CCData::Bool(b) => self.vm.execute_store(StoreBool(b)),
            CCData::List(l) => {
                let item_refs: Vec<Vec<ValueReference>> = l.iter().map(|c| self.store_cc_data(c.clone()).unwrap()).collect::<Vec<Vec<ValueReference>>>();
                let item_refs: Vec<ValueReference> = item_refs.into_iter().flatten().collect();

                self.vm.execute_store(StoreOp::StoreList(item_refs.iter().collect()))
            }
            CCData::Dict(d) => {
                let item_refs: Vec<(String, Vec<ValueReference>)> = d.iter().map(|(k, v)| (k.clone(), self.store_cc_data(v.clone()).unwrap())).collect::<Vec<(String, Vec<ValueReference>)>>();
                let item_refs: HashMap<String, ValueReference> = item_refs.into_iter().map(|(k, v)| (k, v[0].clone())).collect();
                let item_refs: HashMap<String, &ValueReference> = item_refs.iter().map(|(k, v)| (k.clone(), v)).collect();

                self.vm.execute_store(StoreOp::StoreDict(item_refs))

            }
        }
    }

    fn fill_collection_skeleton(&self, collection_ref: ValueReference, value: Collection) -> ExecResult<()> {
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
            let import_ref = match self.get(import_path) {
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
            let stored_ref = self.store_cc_data(constant.data.clone())?[0].clone();

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

            // temporarily store the function graph in memory at a random location
            let func_ref = match self.vm.execute_store(StoreOp::StoreFunctionGraph(func.graph, Some(&collection_ref))) {
                Ok(result) => result[0].clone(),
                Err(err) => return Err(format!("Error storing function {} for collection {}: {}", symbol, symbol, err))
            };

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
}

impl<'a> VmInterface for GraphiteApi<'a> {
    fn store_int(&mut self, value: IntLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreOp::StoreInt(value), symbol)
    }

    fn store_float(&mut self, value: FloatLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreOp::StoreFloat(value), symbol)
    }

    fn store_string(&mut self, value: StringLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreOp::StoreString(value), symbol)
    }

    fn store_bool(&mut self, value: BoolLive, symbol: Symbol) -> ExecResult<()> {
        self.store_value(StoreOp::StoreBool(value), symbol)
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
            Some(path) => Some(self.get(path)?),
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
        let func_ref = self.get(func.clone())?;
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