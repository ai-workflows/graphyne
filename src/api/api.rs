use std::collections::HashMap;
use crate::api::interface::{Symbol, VmInterface};
use crate::core::data::live::{LiveData, IntLive, FloatLive, StringLive, BoolLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::nodes::FunctionGraph;
use crate::core::vm::store_op::StoreOp;
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

    fn store_function(&mut self, func: FunctionGraph, symbol: Symbol) -> ExecResult<()> {
        let store_op = StoreOp::StoreFunctionGraph(func);
        let store_result: Vec<ValueReference> = self.vm.execute_store(store_op).unwrap();
        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
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

    fn execute(&mut self, func: Symbol, inputs: Vec<Symbol>, outputs: Vec<Symbol>) -> ExecResult<()> {
        let func_ref = self.symbol_table.get(&func).unwrap().clone();
        let get_func_result: StoredData = self.vm.get_ref_value(&func_ref).unwrap();
        let func_sig = get_func_result.as_live().as_func().unwrap().unwrap();

        // verify that the number of inputs is correct
        if func_sig.input_vals.len() != inputs.len() {
            return Err(format!("Number of inputs does not match number of inputs for function {}.", func));
        }

        let mut input_refs: Vec<ValueReference> = Vec::new();

        for input in inputs {
            let input_ref = self.symbol_table.get(&input).unwrap().clone();
            input_refs.push(input_ref);
        }

        let exec_result: Vec<ValueReference> = match self.vm.handle_call_function(&func_sig, &input_refs) {
            Ok(result) => result,
            Err(err) => return Err(format!("Error executing function {}: {}", func, err)),
        };

        // verify that the number of outputs is correct
        if exec_result.len() != outputs.len() {
            return Err(format!("Expected {} outputs, but got {}", outputs.len(), exec_result.len()));
        }

        // store the outputs
        for (i, output) in outputs.iter().enumerate() {
            let output_ref = exec_result[i].clone();
            self.symbol_table.insert(output.clone(), output_ref);
        }

        Ok(())
    }
}