use std::collections::HashMap;
use crate::binder::Binder;
use crate::binder::functions::FunctionGraph;
use crate::runtime::data::live::{BoolLive, FloatLive, IntLive, PointerLive, StringLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult, Symbol, SymbolPath};
use crate::runtime::mmu::mmu::execute_store;
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::store_op::StoreOp::{StoreBool, StoreFloat, StoreInt, StoreString};
use crate::runtime::mmu::value_ref::ValueReference;

pub trait IBinder {
    fn store_int(&mut self, value: IntLive, symbol: Symbol) -> ExecResult<()>;
    fn store_float(&mut self, value: FloatLive, symbol: Symbol) -> ExecResult<()>;
    fn store_string(&mut self, value: StringLive, symbol: Symbol) -> ExecResult<()>;
    fn store_bool(&mut self, value: BoolLive, symbol: Symbol) -> ExecResult<()>;
    fn store_list(&mut self, values: Vec<Symbol>, symbol: Symbol) -> ExecResult<()>;
    fn store_dict(&mut self, values: HashMap<String, Symbol>, symbol: Symbol) -> ExecResult<()>;
    fn store_function(&mut self, func: FunctionGraph, symbol: Symbol, class_context: Option<SymbolPath>) -> ExecResult<()>;
    fn store_multiple(&mut self, values: Vec<StoreOp>, prefix: Symbol) -> ExecResult<Vec<Symbol>>;
    fn get(&self, symbol: Symbol) -> ExecResult<StoredData>;
    fn get_ptr(&self, symbol: Symbol) -> ExecResult<PointerLive>;
    fn drop(&mut self, symbol: Symbol) -> ExecResult<()>;
}

impl IBinder for Binder {
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
        let store_result: Vec<ValueReference> = execute_store(self.mmu.clone(), store_op).unwrap();
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
        let store_result: Vec<ValueReference> = execute_store(self.mmu.clone(), store_op).unwrap();
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
        let store_result: Vec<ValueReference> = execute_store(self.mmu.clone(), store_op).unwrap();

        drop(context_val);

        self.symbol_table.insert(symbol, store_result[0].clone());
        Ok(())
    }

    fn store_multiple(&mut self, values: Vec<StoreOp>, prefix: Symbol) -> ExecResult<Vec<Symbol>> {
        let mut symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();
        let mut symbols: Vec<Symbol> = Vec::new();

        for (i, value) in values.into_iter().enumerate() {
            let symbol = format!("{}{}", prefix, i);
            let store_result: Vec<ValueReference> = execute_store(self.mmu.clone(), value).unwrap();
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

        self.mmu.get_ref_value(val_ref).map(|stored| stored)
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
}