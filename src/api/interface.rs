use std::collections::HashMap;
use crate::core::data::live::{IntLive, LiveData};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::nodes::FunctionGraph;
use crate::core::vm::ops::Operation;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::{VM};

pub type Symbol = String;

pub fn store_int<'a>(value: IntLive, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let store_op = Operation::StoreInt(value);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_float<'a>(value: f64, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let store_op = Operation::StoreFloat(value);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_string<'a>(value: String, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let store_op = Operation::StoreString(value);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_bool<'a>(value: bool, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let store_op = Operation::StoreBool(value);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_list<'a>(values: Vec<Symbol>, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let mut value_refs: Vec<&ValueReference> = Vec::new();

    for value in values {
        let value_ref = symbol_table.get(&value).unwrap();
        value_refs.push(value_ref);
    }

    let store_op = Operation::StoreList(value_refs);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_dict<'a>(values: HashMap<String, Symbol>, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let mut value_refs: HashMap<String, &ValueReference> = HashMap::new();

    for (key, value) in values {
        let value_ref = symbol_table.get(&value).unwrap();
        value_refs.insert(key, value_ref);
    }

    let store_op = Operation::StoreDict(value_refs);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn store_function<'a>(func: FunctionGraph, symbol: Symbol, vm: &'a VM, symbol_table: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
    let store_op = Operation::StoreFunctionGraph(func);

    let store_result: Vec<ValueReference> = vm.execute_op(store_op).unwrap();

    symbol_table.insert(symbol, store_result[0].clone());

    Ok(())
}

pub fn get(symbol: Symbol, vm: &VM, symbol_table: &mut HashMap<Symbol, ValueReference>) -> ExecResult<StoredData> {
    let value_ref = symbol_table.get(&symbol).unwrap();
    let stored_data = vm.get_ref_value(value_ref).unwrap();

    Ok(stored_data)
}

pub fn execute<'a>(func_id: Symbol, args: Vec<Symbol>, outputs_symbols: Vec<Symbol>, vm: &'a VM, symbol_table: &mut HashMap<String, ValueReference<'a>>) -> ExecResult<()> {
    let mut arg_refs: Vec<ValueReference> = Vec::new();

    for arg in args {
        let arg_ref = symbol_table.get(&arg).unwrap();
        arg_refs.push(arg_ref.clone());
    }

    let func_ref = symbol_table.get(&func_id).unwrap();
    let get_func_result: StoredData = vm.get_ref_value(func_ref).unwrap();
    let func_sig = get_func_result.as_live().as_func().unwrap().unwrap();

    let execute_result: Vec<ValueReference> = vm.handle_call_function(&func_sig, &arg_refs).unwrap();

    // verify that the number of outputs is correct
    if execute_result.len() != outputs_symbols.len() {
        return Err(format!("Expected {} outputs, but got {}", outputs_symbols.len(), execute_result.len()));
    }

    // store the outputs
    for (i, output_symbol) in outputs_symbols.iter().enumerate() {
        let output_ref = execute_result.get(i).unwrap();
        symbol_table.insert(output_symbol.clone(), output_ref.clone());
    }

    Ok(())
}

fn test() {
    let vm = VM::new();
    let mut symbol_table: HashMap<Symbol, ValueReference> = HashMap::new();


    let x = store_int(5, "x".to_string(), &vm, &mut symbol_table).unwrap();
    let y = store_int(10, "y".to_string(), &vm, &mut symbol_table).unwrap();


}