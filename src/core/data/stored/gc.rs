use std::marker::PhantomData;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::{GarbageCollectable, GCObject, GCObjectData, GCObjectType};

impl GarbageCollectable<StoredData> for StoredData {
    /// Gets the gc object data as its live type and then clones into a new stored data object.
    fn from_gc_object(object: &GCObject<StoredData>) -> ExecResult<Self> {
        match object.data_type {
            GCObjectType::Buffer => Ok(StoredData::NullStored.into()),
            GCObjectType::Integer => object.as_int().map(|int_data| StoredData::IntStored(int_data.clone())),
            GCObjectType::Float => object.as_float().map(|float_data| StoredData::FloatStored(float_data.clone())),
            GCObjectType::String => object.as_string().map(|string_data| StoredData::StringStored(string_data.clone())),
            GCObjectType::Bool => object.as_bool().map(|bool_data| StoredData::BoolStored(bool_data.clone())),
            GCObjectType::Pointer => object.as_pointer().map(|pointer_data| StoredData::PointerStored(pointer_data.clone())),
            GCObjectType::List => object.as_list().map(|list_data| StoredData::ListStored(list_data.clone())),
            GCObjectType::Dict => object.as_dict().map(|dict_data| StoredData::DictStored(dict_data.clone())),
            GCObjectType::Func => object.as_func().map(|func_data| StoredData::FuncStored(func_data.clone())),
            GCObjectType::FuncVal => object.as_func_val().map(|func_val_data| StoredData::FuncValStored(func_val_data.clone())),
            GCObjectType::FuncOp => object.as_func_op().map(|func_op_data| StoredData::FuncOpStored(func_op_data.clone())),
        }
    }

    /// Moves the stored data into a new gc object.
    fn to_gc_object(self) -> GCObject<StoredData> {
        match self {
            StoredData::NullStored => {
                let data = GCObjectData::Null;
                GCObject {
                    data_type: GCObjectType::Buffer,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::IntStored(int_live) => {
                let data = GCObjectData::Int(int_live);
                GCObject {
                    data_type: GCObjectType::Integer,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::FloatStored(float_live) => {
                let data = GCObjectData::Float(float_live);
                GCObject {
                    data_type: GCObjectType::Float,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::StringStored(string_live) => {
                let data = GCObjectData::String(string_live);
                GCObject {
                    data_type: GCObjectType::String,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::BoolStored(bool_live) => {
                let data = GCObjectData::Bool(bool_live);
                GCObject {
                    data_type: GCObjectType::Bool,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::PointerStored(gc_pointer) => {
                let data = GCObjectData::Pointer(gc_pointer);
                GCObject {
                    data_type: GCObjectType::Pointer,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::ListStored(list_live) => {
                let data = GCObjectData::List(list_live);
                GCObject {
                    data_type: GCObjectType::List,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::DictStored(dict_live) => {
                let data = GCObjectData::Dict(dict_live);
                GCObject {
                    data_type: GCObjectType::Dict,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::FuncStored(func_live) => {
                let data = GCObjectData::Func(func_live);
                GCObject {
                    data_type: GCObjectType::Func,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::FuncValStored(func_val_live) => {
                let data = GCObjectData::FuncVal(func_val_live);
                GCObject {
                    data_type: GCObjectType::FuncVal,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
            StoredData::FuncOpStored(func_op_live) => {
                let data = GCObjectData::FuncOp(func_op_live);
                GCObject {
                    data_type: GCObjectType::FuncOp,
                    data,
                    ref_count: 0,
                    phantom: PhantomData
                }
            }
        }
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