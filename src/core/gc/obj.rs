use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use crate::core::data::live::{DictLive, FloatLive, FuncLive, FuncOpLive, FuncValLive, IntLive, ListLive, PointerLive, StringLive};
use crate::core::data::live::live_data::BoolLive;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::{GCPointer};

#[derive(PartialEq, Debug, Clone)]
pub enum GCObjectType {
    Buffer,
    Integer,
    Float,
    String,
    Bool,
    Pointer,
    List,
    Dict,
    Func,
    FuncVal,
    FuncOp,
}

#[derive(Debug, Clone)]
pub enum GCObjectData {
    Null,
    Int(IntLive),
    Float(FloatLive),
    String(StringLive),
    Bool(BoolLive),
    Pointer(PointerLive),
    List(ListLive),
    Dict(DictLive),
    Func(FuncLive),
    FuncVal(FuncValLive),
    FuncOp(FuncOpLive),
}

#[derive(Clone)]
pub struct GCObject<T> {
    pub data_type: GCObjectType,
    pub data: GCObjectData,
    pub ref_count: usize,
    pub phantom: PhantomData<T>,
}

impl<T> Debug for GCObject<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCObject")
            .field("data_type", &self.data_type)
            .field("data", &self.data)
            .field("ref_count", &self.ref_count)
            .finish()
    }
}

impl<T> GCObject<T> {
    /// Gets all of the child pointers of this object, including those in lists and dicts.
    pub fn get_pointers(&mut self) -> Vec<&mut GCPointer<StoredData>> {
        let mut result = Vec::new();

        match self.data_type {
            GCObjectType::Pointer => {
                let pointer: &mut GCPointer<StoredData> = self.as_pointer_mut().unwrap();

                result.push(pointer);
            },
            GCObjectType::List => {
                let list: &mut ListLive = self.as_list_mut().unwrap();

                for pointer in list.iter_mut() {
                    result.push(pointer);
                }
            },
            GCObjectType::Dict => {
                let dict: &mut HashMap<String, GCPointer<StoredData>> = self.as_dict_mut().unwrap();

                for pointer in dict.values_mut() {
                    result.push(pointer);
                }
            },
            GCObjectType::Func => {
                let func: &mut FuncLive = self.as_func_mut().unwrap();

                let inputs: &mut Vec<GCPointer<StoredData>> = &mut func.input_vals;
                for pointer in inputs.iter_mut() {
                    result.push(pointer);
                }

                let outputs: &mut Vec<GCPointer<StoredData>> = &mut func.output_vals;
                for pointer in outputs.iter_mut() {
                    result.push(pointer);
                }

                let constants: &mut Vec<GCPointer<StoredData>> = &mut func.constant_vals;
                for pointer in constants.iter_mut() {
                    result.push(pointer);
                }
            },
            GCObjectType::FuncVal => {
                let func_val: &mut FuncValLive = self.as_func_val_mut().unwrap();

                let dependents: &mut Vec<GCPointer<StoredData>> = &mut func_val.dependents;
                for pointer in dependents.iter_mut() {
                    result.push(pointer);
                }

                // only get the constant if it is not a pointer to the class context
                // this will prevent a circular reference between the collection and function
                if !func_val.is_self {
                    let constant: &mut Option<GCPointer<StoredData>> = &mut func_val.constant;
                    if let Some(pointer) = constant {
                        result.push(pointer);
                    }
                }
            },
            GCObjectType::FuncOp => {
                let func_op: &mut FuncOpLive = self.as_func_op_mut().unwrap();

                // Do not include the inputs, as they will be stored as uncounted pointers.
                // This is to avoid a circular reference between the FuncOp and the FuncVal that would prevent them from being collected.
                // let inputs: &mut Vec<GCPointer<StoredData>> = &mut func_op.input_vals;
                // for pointer in inputs.iter_mut() {
                //     result.push(pointer);
                // }

                let output: &mut GCPointer<StoredData> = &mut func_op.output_val;
                result.push(output);
            },
            _ => {}
        }

        result
    }

    // Functions to get a mutable reference to the data in the GCObject as a specific type.

    pub fn as_int(&self) -> ExecResult<&IntLive> {
        if self.data_type == GCObjectType::Integer {
            match &self.data {
                GCObjectData::Int(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_float(&self) -> ExecResult<&FloatLive> {
        if self.data_type == GCObjectType::Float {
            match &self.data {
                GCObjectData::Float(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_string(&self) -> ExecResult<&StringLive> {
        if self.data_type == GCObjectType::String {
            match &self.data {
                GCObjectData::String(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_bool(&self) -> ExecResult<&BoolLive> {
        if self.data_type == GCObjectType::Bool {
            match &self.data {
                GCObjectData::Bool(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_pointer(&self) -> ExecResult<&GCPointer<StoredData>> {
        if self.data_type == GCObjectType::Pointer {
            match &self.data {
                GCObjectData::Pointer(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_pointer_mut(&mut self) -> ExecResult<&mut GCPointer<StoredData>> {
        if self.data_type == GCObjectType::Pointer {
            match &mut self.data {
                GCObjectData::Pointer(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_list(&self) -> ExecResult<&ListLive> {
        if self.data_type == GCObjectType::List {
            match &self.data {
                GCObjectData::List(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_list_mut(&mut self) -> ExecResult<&mut ListLive> {
        if self.data_type == GCObjectType::List {
            match &mut self.data {
                GCObjectData::List(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_dict(&self) -> ExecResult<&DictLive> {
        if self.data_type == GCObjectType::Dict {
            match &self.data {
                GCObjectData::Dict(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_dict_mut(&mut self) -> ExecResult<&mut DictLive> {
        if self.data_type == GCObjectType::Dict {
            match &mut self.data {
                GCObjectData::Dict(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func(&self) -> ExecResult<&FuncLive> {
        if self.data_type == GCObjectType::Func {
            match &self.data {
                GCObjectData::Func(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func_mut(&mut self) -> ExecResult<&mut FuncLive> {
        if self.data_type == GCObjectType::Func {
            match &mut self.data {
                GCObjectData::Func(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func_val(&self) -> ExecResult<&FuncValLive> {
        if self.data_type == GCObjectType::FuncVal {
            match &self.data {
                GCObjectData::FuncVal(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func_val_mut(&mut self) -> ExecResult<&mut FuncValLive> {
        if self.data_type == GCObjectType::FuncVal {
            match &mut self.data {
                GCObjectData::FuncVal(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func_op(&self) -> ExecResult<&FuncOpLive> {
        if self.data_type == GCObjectType::FuncOp {
            match &self.data {
                GCObjectData::FuncOp(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }

    pub fn as_func_op_mut(&mut self) -> ExecResult<&mut FuncOpLive> {
        if self.data_type == GCObjectType::FuncOp {
            match &mut self.data {
                GCObjectData::FuncOp(value) => Ok(value),
                _ => Err("Invalid data type".to_string()),
            }
        } else {
            Err("Invalid data type".to_string())
        }
    }
}