use std::collections::HashMap;
use crate::core::ExecResult;
use crate::core::nodes::fn_graph::FunctionGraph;
use crate::core::nodes::fn_val::ValIdentifier;
use crate::core::vm::ops::Operation;
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {
    /// Stores a function in the VM given its graph representation.
    pub fn store_function(&self, func: &FunctionGraph) -> ExecResult<Vec<ValueReference>> {
        // create buffers for each value node
        let mut values: HashMap<ValIdentifier, ValueReference> = HashMap::new();
        let mut constants: HashMap<ValIdentifier, ValueReference> = HashMap::new();
        let mut constant_vals: Vec<ValueReference> = Vec::new();

        for val in &func.values {


            let val_refs = self.execute_store(StoreOp::CreateBuffer)?;
            let buf = val_refs[0].clone();
            values.insert(val.guid.clone(), buf.clone());

            if let Some(constant) = &val.constant {
                let const_ref = self.store_value(constant.clone())?;
                let const_ref = const_ref[0].clone();
                constants.insert(val.guid.clone(), const_ref);
                constant_vals.push(buf.clone());
            }
            else if let Some(external_ptr) = &val.external {
                // this value is a pointer to an existing external value, convert it to a value reference
                let val_ref = self.value_ref_from_ptr(external_ptr.clone())?;
                constants.insert(val.guid.clone(), val_ref);
                constant_vals.push(buf.clone());
                continue;
            }
        }

        // create a hashmap to track the ops that are dependent on each value
        let mut value_deps_helper: HashMap<ValIdentifier, Vec<usize>> = HashMap::new();

        // create each op
        let mut ops: Vec<ValueReference> = Vec::new();

        for op in &func.ops {
            // get the input values for this op
            let mut input_val_refs: Vec<&ValueReference> = Vec::new();

            for val_id in &op.input_val_ids {
                let val = match values.get(&val_id.to_string()) {
                    Some(val) => val,
                    None => return Err(format!("Value {} does not exist.", val_id)),
                };

                input_val_refs.push(val);
            }

            // get the output value for this op
            let output_val_ref: &ValueReference = match values.get(&op.output_val_id) {
                Some(val) => val,
                None => return Err(format!("Error getting output value for op {}.", op.output_val_id)),
            };

            let store_op = StoreOp::StoreFunctionOp(op.opcode, input_val_refs, output_val_ref);
            let op_refs: Vec<ValueReference> = self.execute_store(store_op)?;  // TODO: input and output refs are not being counted
            let op_ref: ValueReference = op_refs[0].clone();
            ops.push(op_ref);

            // add to the value deps hashmap
            for val_id in &op.input_val_ids {
                let val_deps = value_deps_helper.entry(val_id.clone()).or_insert(Vec::new());
                val_deps.push(ops.len() - 1);
            }
        }

        // finalize the value deps hashmap
        let mut value_deps: HashMap<ValIdentifier, Vec<&ValueReference>> = HashMap::new();

        for (val_id, op_ids) in value_deps_helper.iter() {
            let mut val_deps: Vec<&ValueReference> = Vec::new();

            for op_id in op_ids {
                let op_ref = &ops[*op_id];
                val_deps.push(op_ref);
            }

            value_deps.insert(val_id.clone(), val_deps);
        }

        // fill the buffers for each value, including the dependent ops
        for (val_id, val_ref) in values.iter() {
            let val_deps: &Vec<&ValueReference>;
            let empty_vec: Vec<&ValueReference>;
            if !value_deps.contains_key(&val_id.to_string()) {
                empty_vec = Vec::new();
                val_deps = &empty_vec;
            }
            else {
                val_deps = value_deps.get(&val_id.to_string()).unwrap();
            }

            // check if the value is a constant
            let const_ref: Option<&ValueReference> = match constants.get(&val_id.to_string()) {
                Some(const_ref) => Some(const_ref),
                None => None,
            };

            // use the deps to store the func val in memory
            let store_op = StoreOp::StoreFunctionVal(val_deps.clone(), const_ref);
            let stored_val_refs = self.execute_store(store_op)?;
            let stored_val_ref = stored_val_refs[0].clone();

            // get the value of the func val that was just stored
            let val_stored = self.get_ref_value(&stored_val_ref)?;

            // fill the buffer with the value
            let fill_buf_op = Operation::SetBuffer(&val_ref, val_stored);
            self.execute_op(fill_buf_op)?;
        }

        // get the refs to the input and output values
        let mut input_val_refs: Vec<&ValueReference> = Vec::new();
        let mut output_val_refs: Vec<&ValueReference> = Vec::new();

        for val_id in &func.input_val_ids {
            let val_ref = values.get(&val_id.to_string()).unwrap();
            input_val_refs.push(val_ref);
        }

        for val_id in &func.output_val_ids {
            let val_ref = values.get(&val_id.to_string()).unwrap();
            output_val_refs.push(val_ref);
        }

        // convert the constant refs to a vector
        let constant_refs: Vec<&ValueReference> = constant_vals.iter().collect();

        // store the function in memory
        let store_func_op = StoreOp::StoreFunction(input_val_refs, output_val_refs, constant_refs);
        let stored_func_refs = self.execute_store(store_func_op)?;
        let stored_func_ref = stored_func_refs[0].clone();

        // return the reference to the function
        Ok(vec![stored_func_ref.clone()])
    }
}