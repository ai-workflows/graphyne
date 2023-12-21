use std::collections::HashMap;
use crate::api::functions::FunctionGraph;
use crate::core::{ExecResult, Symbol};
use crate::core::data::live::{LiveData};
use crate::core::data::stored::StoredData;
use crate::core::vm::ops::Operation;
use crate::core::vm::store_op::StoreOp;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

impl VM {

    fn buffer_fn_values(&self, func: &FunctionGraph) -> ExecResult<(HashMap<Symbol, ValueReference>, HashMap<Symbol, ValueReference>, Vec<ValueReference>)> {
        // stores the buffers for each value in the function
        let mut values = HashMap::new();

        // stores references to the actual value that each constant value represents
        let mut constants = HashMap::new();

        // a list of the buffers that are for constants
        let mut constant_vals = Vec::new();

        for val in &func.values {
            if values.contains_key(&val.symbol) {
                return Err(format!("Symbol {} already exists, ensure that all symbols are unique.", val.symbol));
            }

            // create the buffer
            let val_refs = self.execute_store(StoreOp::CreateBuffer)?;
            let buf = val_refs[0].clone();
            values.insert(val.symbol.clone(), buf.clone());

            // store the constant value if it exists
            if let Some(constant) = &val.constant {
                let const_ref = match constant {
                    StoredData::PointerStored(ptr) => self.value_ref_from_ptr(ptr.clone())?,
                    _ => self.store_value(constant.clone())?.pop().ok_or("Failed to store constant value")?.clone(),
                };

                constants.insert(val.symbol.clone(), const_ref.clone());
                constant_vals.push(buf);
            }
        }

        Ok((values, constants, constant_vals))
    }

    // fn store_fn_ops(&self, func: &FunctionGraph, values: &HashMap<Symbol, ValueReference>) -> ExecResult<(Vec<ValueReference>, HashMap<Symbol, Vec<usize>>)> {
    //     // create a hashmap to track the ops that are dependent on each value
    //     let mut value_deps_helper: HashMap<Symbol, Vec<usize>> = HashMap::new();
    //
    //     // create each op
    //     let mut ops: Vec<ValueReference> = Vec::new();
    //
    //     for op in &func.ops {
    //         // get the input values for this op
    //         let mut input_val_refs: Vec<&ValueReference> = Vec::new();
    //
    //         for val_id in &op.input_vals {
    //             let val = match values.get(&val_id.to_string()) {
    //                 Some(val) => val,
    //                 None => return Err(format!("Input value {} for op {} does not exist. Ensure that the value is defined.", val_id, op.opcode)),
    //             };
    //
    //             input_val_refs.push(val);
    //         }
    //
    //         // get the output values for this op
    //         let mut output_val_refs: Vec<&ValueReference> = Vec::new();
    //         for val_id in &op.output_vals {
    //             let val = match values.get(&val_id.to_string()) {
    //                 Some(val) => val,
    //                 None => return Err(format!("Output value {} for op {} does not exist. Ensure that the value is defined.", val_id, op.opcode)),
    //             };
    //
    //             output_val_refs.push(val);
    //         }
    //
    //         let store_op = StoreOp::StoreFunctionOp(op.opcode, input_val_refs, output_val_refs);
    //         let op_refs: Vec<ValueReference> = self.execute_store(store_op)?;  // TODO: input and output refs are not being counted
    //         let op_ref: ValueReference = op_refs[0].clone();
    //         ops.push(op_ref);
    //
    //         // add to the value deps hashmap
    //         for val_id in &op.input_vals {
    //             let val_deps = value_deps_helper.entry(val_id.clone()).or_insert(Vec::new());
    //             val_deps.push(ops.len() - 1);
    //         }
    //     }
    //
    //     // finalize the value deps hashmap
    //     let mut value_deps: HashMap<Symbol, Vec<&ValueReference>> = HashMap::new();
    //
    //     for (val_id, op_ids) in value_deps_helper.iter() {
    //         let mut val_deps: Vec<&ValueReference> = Vec::new();
    //
    //         for op_id in op_ids {
    //             let op_ref = &ops[*op_id];
    //             val_deps.push(op_ref);
    //         }
    //
    //         value_deps.insert(val_id.clone(), val_deps);
    //     }
    //
    //     Ok((ops, value_deps))
    // }


    /// Stores a function in the VM given its graph representation.
    /// func: The function graph to store.
    /// class_context: A reference to the class (as a dict) that the func belongs to (if any).
    pub fn store_function(&self, func: &FunctionGraph, class_context: Option<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
        // create buffers for each value node
        let (mut values, mut constants, mut constant_vals) = self.buffer_fn_values(func)?;

        // if a class context was provided, store a buffer for the func val that will represent the self reference
        if let Some(class_context) = class_context {
            // create the buffer
            let buf = self.execute_store(StoreOp::CreateBuffer)?;
            let buf = buf[0].clone();

            // store it in the values hashmap with the symbol "self"
            if values.contains_key(&Symbol::from("self")) {
                return Err("Symbol self already exists, ensure that all symbols are unique.".to_string());
            }
            values.insert(Symbol::from("self"), buf.clone());

            // represent the self reference as a constant. This way it has a pointer to the class.
            constants.insert(Symbol::from("self"), class_context.clone());
            constant_vals.push(buf.clone());
        }

        // create a hashmap to track the ops that are dependent on each value
        let mut value_deps_helper: HashMap<Symbol, Vec<usize>> = HashMap::new();

        // create each op
        let mut ops: Vec<ValueReference> = Vec::new();

        for op in &func.ops {
            // get the input values for this op
            let mut input_val_refs: Vec<&ValueReference> = Vec::new();

            for val_id in &op.input_vals {
                let val = match values.get(&val_id.to_string()) {
                    Some(val) => val,
                    None => return Err(format!("Input value {} for op {} does not exist. Ensure that the value is defined.", val_id, op.opcode)),
                };

                input_val_refs.push(val);
            }

            // get the output values for this op
            let mut output_val_refs: Vec<&ValueReference> = Vec::new();
            for val_id in &op.output_vals {
                let val = match values.get(&val_id.to_string()) {
                    Some(val) => val,
                    None => return Err(format!("Output value {} for op {} does not exist. Ensure that the value is defined.", val_id, op.opcode)),
                };

                output_val_refs.push(val);
            }

            let store_op = StoreOp::StoreFunctionOp(op.opcode, input_val_refs, output_val_refs);
            let op_refs: Vec<ValueReference> = self.execute_store(store_op)?;  // TODO: input and output refs are not being counted
            let op_ref: ValueReference = op_refs[0].clone();
            // let op_val = self.get_ref_value(&op_ref)?.as_live().as_func_op()?.ok_or("Failed to get func op")?;
            ops.push(op_ref);

            // add to the value deps hashmap
            for val_id in &op.input_vals {
                let val_deps = value_deps_helper.entry(val_id.clone()).or_insert(Vec::new());
                val_deps.push(ops.len() - 1)
            }
        }

        // finalize the value deps hashmap
        let mut value_deps: HashMap<Symbol, Vec<&ValueReference>> = HashMap::new();

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

            // check if the value is a constant and get the pointer to its value if it is
            let const_ref: Option<&ValueReference> = match constants.get(&val_id.to_string()) {
                Some(const_ref) => Some(const_ref),
                None => None,
            };

            // check if the val_id is "self" and the class context is not None
            let is_self: bool = val_id.to_string() == "self" && class_context.is_some();

            // use the deps to store the func val in memory
            let store_op = StoreOp::StoreFunctionVal(val_deps.clone(), const_ref, is_self);
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

        for val_id in &func.input_vals {
            let val_ref = values.get(&val_id.to_string()).unwrap();
            input_val_refs.push(val_ref);
        }

        for val_id in &func.output_vals {
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