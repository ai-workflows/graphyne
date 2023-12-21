use std::collections::{HashMap, HashSet};
use crate::core::data::live::{LiveData, FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult, Symbol};
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    /// Helper function to get a function value from a pointer.
    fn get_func_val(&self, ptr: &PointerLive) -> ExecResult<FuncValLive> {
        let val = match self.state.read().unwrap().get(ptr) {
            Ok(val) => val,
            Err(msg) => return Err(format!("Cannot find func val (pointer id: {}) for function: {}", ptr.id, msg))
        };
        let func_val = match val {
            StoredData::FuncValStored(func_val) => func_val,
            _ => {
                let type_code = val.type_code()?;
                return Err(format!("Expected func val but got type: {}", type_code));
            }
        };
        Ok(func_val)
    }

    /// Handles the call of an operation that is part of a function.
    pub fn handle_call_function_op(&self, func_op: &FuncOpLive, context: &HashMap<String, ValueReference>) -> ExecResult<Vec<ValueReference>> {
        let arg_values = self.get_func_op_args(func_op, context)?;
        let op = func_op.opcode.to_operation(&arg_values);
        self.execute_op(op)
    }

    fn get_func_op_args<'a>(&'a self, func_op: &FuncOpLive, context: &'a HashMap<String, ValueReference>) -> ExecResult<Vec<&ValueReference>> {
        func_op.input_vals.iter()
            .map(|arg_ptr| self.resolve_func_op_arg(arg_ptr, context))
            .collect()
    }

    fn resolve_func_op_arg<'a>(&'a self, arg_ptr: &PointerLive, context: &'a HashMap<String, ValueReference<'a>>) -> ExecResult<&ValueReference> {
        let func_val = self.get_func_val(arg_ptr)?;
        context.get(&func_val.guid)
            .ok_or_else(|| format!("Function cannot find input value: {}", func_val.guid))
    }

    fn handle_func_val_dependents(&self, func_val: &FuncValLive, op_queue: &mut Vec<FuncOpLive>) -> ExecResult<()> {
        for dependent_op in &func_val.dependents {
            let state = self.state.read().unwrap();
            let dependent_op_val_live = state.get(dependent_op)
                .map_err(|msg| format!("Function cannot find dependent operation: {}", msg))?
                .as_live().as_func_op()
                .ok_or_else(|| "Function cannot execute a non-func-op value".to_string())?
                .map_err(|msg| format!("Function cannot execute a non-func-op value: {}", msg))?;

            op_queue.push(dependent_op_val_live);
        }
        Ok(())
    }


    fn initialize_func_call<'a>(&'a self, func: &FuncLive, args: &[ValueReference<'a>], context: &mut HashMap<Symbol, ValueReference<'a>>, op_queue: &mut Vec<FuncOpLive>) -> ExecResult<()> {
        self.bind_args_to_context(func, args, context, op_queue)?;
        self.handle_constants(func, context, op_queue)?;
        Ok(())
    }

    fn bind_args_to_context<'a>(&'a self, func: &FuncLive, args: &[ValueReference<'a>], context: &mut HashMap<Symbol, ValueReference<'a>>, op_queue: &mut Vec<FuncOpLive>) -> ExecResult<()> {
        for (i, arg_value) in args.iter().enumerate() {
            let input_ptr = func.input_vals.get(i)
                .ok_or("Function input value missing")?;
            let input_val_live = self.get_func_val(input_ptr)
                .map_err(|msg| format!("Function cannot get input value: {}", msg))?;

            context.insert(input_val_live.guid.clone(), arg_value.clone());
            self.handle_func_val_dependents(&input_val_live, op_queue)?;
        }
        Ok(())
    }

    fn handle_constants<'a>(&'a self, func: &FuncLive, context: &mut HashMap<Symbol, ValueReference<'a>>, op_queue: &mut Vec<FuncOpLive>) -> ExecResult<()> {
        for constant_ptr in &func.constant_vals {
            let constant_val = self.get_func_val(constant_ptr)
                .map_err(|msg| format!("Function cannot get constant value: {}", msg))?;

            let constant_ref = match constant_val.constant.as_ref() {
                Some(ptr) => self.value_ref_from_ptr(ptr.clone())?,
                None => return Err("Function expected constant but none found".to_string())
            };

            context.insert(constant_val.guid.clone(), constant_ref);
            self.handle_func_val_dependents(&constant_val, op_queue)?;
        }
        Ok(())
    }


    fn try_execute_fn_op<'a>(&'a self, op: &FuncOpLive, context: &mut HashMap<Symbol, ValueReference<'a>>) -> ExecResult<Vec<ValueReference>> {
        self.validate_op_inputs(op, context)?;
        let result_val_refs = self.handle_call_function_op(op, context)
            .map_err(|msg| format!("Execution of operation {} failed: {}", op.opcode, msg))?;

        if result_val_refs.len() != op.output_vals.len() {
            return Err(format!("Function operation expected {} result values, but got {}", op.output_vals.len(), result_val_refs.len()));
        }

        Ok(result_val_refs)
    }

    fn validate_op_inputs<'a>(&'a self, op: &FuncOpLive, context: &HashMap<Symbol, ValueReference<'a>>) -> ExecResult<()> {
        op.input_vals.iter().enumerate().try_for_each(|(arg_index, input_ptr)| {
            let input_val = self.get_func_val(input_ptr)
                .map_err(|msg| format!("Function cannot get input value (index {}): {}", arg_index, msg))?;

            if !context.contains_key(&input_val.guid) {
                Err(format!("Operation {} cannot be executed because input value index {} is not known", op.opcode, arg_index))
            } else {
                Ok(())
            }
        })
    }

    /// Handles the call of a function.
    pub fn handle_call_function<'a>(&'a self, func: &FuncLive, args: &[ValueReference<'a>]) -> ExecResult<Vec<ValueReference<'a>>> {
        // Check if the function has the right number of args
        if func.input_vals.len() != args.len() {
            return Err(format!("Function expected {} arguments, but got {}", func.input_vals.len(), args.len()));
        }

        let mut context = HashMap::new();
        let mut op_queue = Vec::new();
        let mut executed = HashSet::new();

        // Initialize the function call
        self.initialize_func_call(func, args, &mut context, &mut op_queue)?;

        while let Some(op) = op_queue.pop() {
            if executed.contains(&op.guid) {
                continue;
            }

            if let Ok(result_val_refs) = self.try_execute_fn_op(&op, &mut context) {
                for (output_ptr, result_val_ref) in op.output_vals.iter().zip(&result_val_refs) {
                    let output_val = self.get_func_val(output_ptr)
                        .map_err(|msg| format!("Function cannot get output value for operation {}: {}", op.opcode, msg))?;
                    context.insert(output_val.guid.clone(), result_val_ref.clone());
                    self.handle_func_val_dependents(&output_val, &mut op_queue)?;
                }
                executed.insert(op.guid.clone());
            }
        }

        func.output_vals.iter()
            .map(|output_ptr| {
                let output_val = self.get_func_val(output_ptr)
                    .map_err(|msg| format!("Function cannot get output value for function: {}", msg))?;
                context.get(&output_val.guid)
                    .cloned()
                    .ok_or_else(|| "Function cannot find return value in context".to_string())
            })
            .collect()
    }
}