use std::collections::HashMap;
use crate::core::data::live::{BoolLive, FloatLive, IntLive, PointerLive, StringLive};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::nodes::FunctionGraph;
use crate::core::vm::store_op::StoreOp;

pub type Symbol = String;

pub trait VmInterface {
    fn store_int(&mut self, value: IntLive, symbol: Symbol) -> ExecResult<()>;
    fn store_float(&mut self, value: FloatLive, symbol: Symbol) -> ExecResult<()>;
    fn store_string(&mut self, value: StringLive, symbol: Symbol) -> ExecResult<()>;
    fn store_bool(&mut self, value: BoolLive, symbol: Symbol) -> ExecResult<()>;
    fn store_list(&mut self, values: Vec<Symbol>, symbol: Symbol) -> ExecResult<()>;
    fn store_dict(&mut self, values: HashMap<String, Symbol>, symbol: Symbol) -> ExecResult<()>;
    fn store_function(&mut self, func: FunctionGraph, symbol: Symbol) -> ExecResult<()>;
    fn store_multiple(&mut self, values: Vec<StoreOp>, prefix: Symbol) -> ExecResult<Vec<Symbol>>;
    fn get(&self, symbol: Symbol) -> ExecResult<StoredData>;
    fn get_ptr(&self, symbol: Symbol) -> ExecResult<PointerLive>;
    fn drop(&mut self, symbol: Symbol) -> ExecResult<()>;
    fn execute(&mut self, func: Symbol, inputs: Vec<Symbol>, outputs: Vec<Symbol>) -> ExecResult<()>;
}