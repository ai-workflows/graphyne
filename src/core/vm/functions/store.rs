use std::collections::HashMap;
use crate::api::functions::FunctionGraph;
use crate::core::{ExecResult, Symbol};
use crate::core::data::stored::StoredData;
use crate::core::vm::ops::Operation;
use crate::core::vm::store::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    fn store_constant_value(&self, constant: &StoredData) -> ExecResult<ValueReference> {
        match constant {
            StoredData::PointerStored(ptr) => self.value_ref_from_ptr(ptr.clone()),
            _ => self.store_value(constant.clone())?
                .into_iter()
                .last()
                .ok_or("Failed to store constant value".to_string()),
        }
    }

    fn validate_symbol(&self, values: &HashMap<String, ValueReference>, symbol: &Symbol) -> ExecResult<()> {
        match values.get(symbol) {
            Some(_) => Err(format!("Duplicate symbol detected: {}. Ensure all symbols are unique.", symbol)),
            None => Ok(()),
        }
    }

    fn create_buffer(&self) -> ExecResult<ValueReference> {
        let buf = self.execute_store(StoreOp::CreateBuffer)?
            .get(0)
            .ok_or("Buffer creation failed")?
            .clone();

        Ok(buf)
    }


    fn buffer_fn_values(&self, func: &FunctionGraph) -> ExecResult<(HashMap<Symbol, ValueReference>, HashMap<Symbol, ValueReference>, Vec<ValueReference>)> {
        let mut values = HashMap::new();
        let mut constants = HashMap::new();
        let mut constant_vals = Vec::new();

        for val in &func.values {
            self.validate_symbol(&values, &val.symbol)?;
            let buf = self.create_buffer()?;
            values.insert(val.symbol.clone(), buf.clone());

            // Handle constant value, if present
            if let Some(constant) = &val.constant {
                let const_ref = self.store_constant_value(constant)?;

                constants.insert(val.symbol.clone(), const_ref);
                constant_vals.push(buf); // Store buffer for constant
            }
        }

        Ok((values, constants, constant_vals))
    }


    fn store_fn_ops<'a>(&'a self, func: &FunctionGraph, values: &HashMap<Symbol, ValueReference>) -> ExecResult<HashMap<Symbol, Vec<ValueReference>>> {
        let mut ops = Vec::new();
        let mut value_deps_helper: HashMap<String, Vec<usize>> = HashMap::new();

        for op in &func.ops {
            // Retrieve input and output value references, returning an error if any are missing
            let input_val_refs: Result<Vec<&ValueReference>, String> = op.input_vals.iter()
                .map(|val_id|
                    values.get(&val_id.to_string())
                        .ok_or_else(|| format!("Input value {} for op {} does not exist.", val_id, op.opcode)))
                .collect();

            let output_val_refs: Result<Vec<&ValueReference>, String> = op.output_vals.iter()
                .map(|val_id|
                    values.get(&val_id.to_string())
                        .ok_or_else(|| format!("Output value {} for op {} does not exist.", val_id, op.opcode)))
                .collect();

            // Store the operation
            let store_op = StoreOp::StoreFunctionOp(op.opcode, input_val_refs?, output_val_refs?);
            let op_ref = self.execute_store(store_op)?.into_iter().next().ok_or("Failed to store operation")?;
            ops.push(op_ref.clone());

            // Track dependencies
            op.input_vals.iter().for_each(|val_id| {
                value_deps_helper.entry(val_id.clone()).or_default().push(ops.len() - 1);
            });
        }

        let value_deps = value_deps_helper.into_iter().map(|(val_id, op_ids)| {
            let val_deps = op_ids.iter().map(|&op_id| ops[op_id].clone()).collect();
            (val_id, val_deps)
        }).collect();

        Ok(value_deps)
    }

    fn fill_fn_val_buffers(&self, values: &HashMap<Symbol, ValueReference>, value_deps: &HashMap<Symbol, Vec<ValueReference>>, constants: &HashMap<Symbol, ValueReference>, has_context: bool) -> ExecResult<()> {
        for (val_id, val_ref) in values.iter() {
            // Determine dependencies for the value
            let empty_deps = Vec::new();
            let val_deps = value_deps.get(&val_id.to_string()).unwrap_or(&empty_deps);

            // Check if the value is a constant
            let const_ref = constants.get(&val_id.to_string());

            // Determine if the current value is 'outer' and has context
            let is_self = val_id == "outer" && has_context;

            // Store the function value in memory
            let store_op = StoreOp::StoreFunctionVal(val_deps.iter().collect(), const_ref, is_self, Some(val_id.clone()));
            let stored_val_ref = self.execute_store(store_op)?.get(0).cloned().ok_or("Failed to store function value")?;

            // Get the stored function value
            let val_stored = self.get_ref_value(&stored_val_ref)?;

            // Fill the buffer with the retrieved value
            let fill_buf_op = Operation::SetBuffer(val_ref, val_stored);
            self.execute_op(fill_buf_op)?;
        }

        Ok(())
    }


    fn finalize_store_function(&self, func: &FunctionGraph, values: &HashMap<Symbol, ValueReference>, constant_vals: &[ValueReference]) -> ExecResult<ValueReference> {
        // Collect input and output value references
        let input_val_refs: Result<Vec<_>, _> = func.input_vals.iter()
            .map(|val_id| values.get(&val_id.to_string()).ok_or_else(|| format!("Missing input value reference: {}", val_id)))
            .collect();

        let output_val_refs: Result<Vec<_>, _> = func.output_vals.iter()
            .map(|val_id| values.get(&val_id.to_string()).ok_or_else(|| format!("Missing output value reference: {}", val_id)))
            .collect();

        // Store the function in memory
        let store_func_op = StoreOp::StoreFunction(input_val_refs?, output_val_refs?, constant_vals.iter().collect());
        let stored_func_ref = self.execute_store(store_func_op)?.into_iter().next().ok_or("Failed to store function")?;

        // Return the reference to the stored function
        Ok(stored_func_ref)
    }


    /// Stores a function in the VM given its graph representation.
    /// func: The function graph to store.
    /// class_context: A reference to the class (as a dict) that the func belongs to (if any).
    pub fn store_function(&self, func: &FunctionGraph, class_context: Option<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
        // Create buffers for values and constants
        let (mut values, mut constants, mut constant_vals) = self.buffer_fn_values(func)?;

        // Handle class context
        if let Some(class_context) = class_context {
            // store the class context as a constant with the symbol 'ctx'
            let self_symbol = Symbol::from("outer");
            self.validate_symbol(&values, &self_symbol)?;
            let buf = self.create_buffer()?;
            values.insert(self_symbol.clone(), buf.clone());
            constants.insert(self_symbol, class_context.clone());
            constant_vals.push(buf);
        }

        // Store operations and their dependencies
        let value_deps = self.store_fn_ops(func, &values)?;

        // Fill buffers for values including dependencies
        self.fill_fn_val_buffers(&values, &value_deps, &constants, class_context.is_some())?;

        // Finalize the function and return the reference
        let stored_func_ref = self.finalize_store_function(func, &values, &constant_vals)?;

        Ok(vec![stored_func_ref])
    }
}