use std::marker::PhantomData;
use std::sync::Arc;
use crate::runtime::data::stored::StoredData;
use crate::runtime::ExecResult;
use crate::runtime::gc::{GarbageCollectable, GCObject, GCPointer};

impl GarbageCollectable<StoredData> for StoredData {
    /// Gets the gc object data as its live type and then clones into a new stored data object.
    fn clone_from_gc_object(object: &GCObject<StoredData>) -> ExecResult<Self> {
        Ok(object.data.clone())
    }

    /// Moves the stored data into a new gc object.
    fn to_gc_object(self) -> GCObject<Arc<StoredData>> {
        GCObject {
            data: Arc::new(self),
            ref_count: 0,
            phantom: PhantomData
        }
    }

    fn from_gc_object(object: &GCObject<StoredData>) -> ExecResult<&Self> {
        Ok(&object.data)
    }

    fn get_pointers(&self) -> Vec<&GCPointer<StoredData>> where StoredData: GarbageCollectable<StoredData> {
        let mut result = Vec::new();

        match self {
            StoredData::PointerStored(ptr) => {
                result.push(ptr);
            },
            StoredData::ListStored(list) => {
                for pointer in list.iter() {
                    result.push(pointer);
                }
            },
            StoredData::DictStored(dict) => {
                for pointer in dict.values() {
                    result.push(pointer);
                }
            },
            StoredData::FuncStored(func) => {
                for pointer in func.input_vals.iter() {
                    result.push(pointer);
                }

                for pointer in func.output_vals.iter() {
                    result.push(pointer);
                }

                for pointer in func.constant_vals.iter() {
                    result.push(pointer);
                }
            },
            StoredData::FuncValStored(func_val) => {
                for pointer in func_val.dependents.iter() {
                    result.push(pointer);
                }

                if !func_val.is_self {
                    if let Some(pointer) = &func_val.constant {
                        result.push(pointer);
                    }
                }
            },
            StoredData::FuncOpStored(func_op) => {
                for pointer in func_op.output_vals.iter() {
                    result.push(pointer);
                }

                // Do not include the inputs, as they will be stored as uncounted pointers.
                // This is to avoid a circular reference between the FuncOp and the FuncVal that would prevent them from being collected.
                // let inputs: &Vec<GCPointer<StoredData>> = &func_op.input_vals;
                // for pointer in inputs.iter() {
                //     result.push(pointer);
                // }
            },
            StoredData::ObjectStored(object) => {
                result.push(&object.type_ptr);

                for pointer in object.fields.values() {
                    result.push(pointer);
                }
            },
            _ => {}
        }

        result
    }

    fn get_pointers_mut(&mut self) -> Vec<&mut GCPointer<StoredData>> {
        let mut result = Vec::new();

        match self {
            StoredData::PointerStored(ptr) => {
                result.push(ptr);
            },
            StoredData::ListStored(list) => {
                for pointer in list.iter_mut() {
                    result.push(pointer);
                }
            },
            StoredData::DictStored(dict) => {
                for pointer in dict.values_mut() {
                    result.push(pointer);
                }
            },
            StoredData::FuncStored(func) => {
                for pointer in func.input_vals.iter_mut() {
                    result.push(pointer);
                }

                for pointer in func.output_vals.iter_mut() {
                    result.push(pointer);
                }

                for pointer in func.constant_vals.iter_mut() {
                    result.push(pointer);
                }
            },
            StoredData::FuncValStored(func_val) => {
                for pointer in func_val.dependents.iter_mut() {
                    result.push(pointer);
                }

                if !func_val.is_self {
                    if let Some(pointer) = &mut func_val.constant {
                        result.push(pointer);
                    }
                }
            },
            StoredData::FuncOpStored(func_op) => {
                for pointer in func_op.output_vals.iter_mut() {
                    result.push(pointer);
                }

                // Do not include the inputs, as they will be stored as uncounted pointers.
                // This is to avoid a circular reference between the FuncOp and the FuncVal that would prevent them from being collected.
                // let inputs: &mut Vec<GCPointer<StoredData>> = &mut func_op.input_vals;
                // for pointer in inputs.iter_mut() {
                //     result.push(pointer);
                // }
            },
            StoredData::ObjectStored(object) => {
                result.push(&mut object.type_ptr);

                for pointer in object.fields.values_mut() {
                    result.push(pointer);
                }
            },
            _ => {}
        }

        result
    }

    // fn get_pointers(&mut self) -> Vec<&mut PointerLive> {
    //     let mut ptrs = Vec::new();
    //
    //     match self {
    //         StoredData::PointerStored(ptr) => {
    //             ptrs.push(ptr);
    //         }
    //         StoredData::ListStored(list) => {
    //             for item in list {
    //                 ptrs.push(item);
    //             }
    //         }
    //         StoredData::DictStored(dict) => {
    //             for item in dict.values_mut() {
    //                 ptrs.push(item);
    //             }
    //         }
    //         StoredData::FuncStored(func) => {
    //             let inputs = &mut func.input_vals;
    //             for input in inputs {
    //                 ptrs.push(input);
    //             }
    //
    //             let outputs = &mut func.output_vals;
    //             for output in outputs {
    //                 ptrs.push(output);
    //             }
    //         }
    //         StoredData::FuncValStored(func_val) => {
    //             let dependents = &mut func_val.dependents;
    //             for dependent in dependents {
    //                 ptrs.push(dependent);
    //             }
    //         }
    //         StoredData::FuncOpStored(func_op) => {
    //             let inputs = &mut func_op.input_vals;
    //             for input in inputs {
    //                 ptrs.push(input);
    //             }
    //
    //             let output: &mut PointerLive = &mut func_op.output_val;
    //             ptrs.push(output);
    //         }
    //         _ => {}
    //     }
    //
    //     ptrs
    // }
}