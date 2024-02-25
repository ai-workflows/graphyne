use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::data::live::{LiveData, PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult};
use crate::runtime::mmu::mmu::{execute_store, MMU};
use crate::runtime::mmu::store_op::StoreOp;
use crate::runtime::mmu::value_ref::ValueReference;

pub fn execute_init(mmu: Arc<MMU>, obj_type_ref: &ValueReference, args: Vec<&ValueReference>) -> ExecResult<Vec<ValueReference>> {
    let obj_type_arc = mmu.get_ref_value(obj_type_ref)?;

    let obj_type = match obj_type_arc.as_ref() {
        StoredData::TypeStored(t) => t,
        _ => return Err("Cannot execute operation init for non-type value".to_string())
    };

    // make sure it is a custom type
    let obj_type = match obj_type {
        TypeLive::Custom(t_name, t_guid, t_fields) => (t_name, t_guid, t_fields),
        _ => return Err("Cannot execute operation init for non-custom type".to_string())
    };

    if obj_type.2.len() != args.len() {
        return Err(format!("Cannot initialize object of type {} with {} arguments, expected {}", obj_type.0, args.len(), obj_type.2.len()));
    }

    let mut vals: HashMap<String, &ValueReference> = HashMap::new();

    for (i, field) in obj_type.2.iter().enumerate() {
        // get the expected type of the field
        let expected_type_ptr: &PointerLive = &field.1;
        let expected_type = mmu.get_ptr_value(expected_type_ptr)?;
        let expected_type = match expected_type.as_ref() {
            StoredData::TypeStored(t) => t,
            _ => return Err(format!("Cannot initialize object of type {}, cannot get type of field {}", field.0, field.0))
        };

        let arg = &args[i];
        let arg_value: Arc<StoredData> = mmu.get_ref_value(arg)?;

        // do a type check if it isn't dynamic
        match expected_type {
            TypeLive::Dynamic => (),
            _ => {
                let arg_type_ptr = match arg_value.as_live().type_of(&mmu.primitive_types) {
                    Some(Ok(ptr)) => ptr,
                    Some(Err(msg)) => return Err(format!("Could not get type of argument {}: {}", i, msg)),
                    None => return Err(format!("Cannot initialize object with argument {} of unknown type", i))
                };

                let arg_type_ref = mmu.get_ptr_value(&arg_type_ptr)?;
                let arg_type: &TypeLive = match arg_type_ref.as_ref() {
                    StoredData::TypeStored(t) => t,
                    _ => return Err(format!("Cannot initialize object with argument {} of non-type value", i))
                };

                if arg_type != expected_type {
                    return Err(format!("Cannot initialize object of type {} with argument {} of type {}, expected {}", obj_type.0, i, arg_type.get_name(), expected_type.get_name()));
                }
            }
        }

        vals.insert(field.0.clone(), arg);
    }

    let store_op: StoreOp = StoreOp::StoreObject(obj_type_ref, vals);
    execute_store(mmu, store_op)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::runtime::data::live::{TypeLive, PointerLive};
//     use crate::runtime::data::stored::StoredData;
//     use crate::runtime::vm::mmu::store_op::StoreOp;
//     use crate::runtime::vm::value_ref::ValueReference;
//     use crate::runtime::vm::VM;
//     use std::intermediate::HashMap;
//     use crate::runtime::Symbol;
//
//     #[test]
//     fn test_execute_init() {
//         let vm = VM::new(2, 2);
//
//         let type_name: Symbol = "TestType".into();
//         let mut type_fields: Vec<(Symbol, &ValueReference)> = vec![];
//
//         let int_type = vm.get_primitive_type(&TypeLive::Integer).unwrap();
//         let string_type = vm.get_primitive_type(&TypeLive::String).unwrap();
//
//         type_fields.push(("name".into(), &string_type));
//         type_fields.push(("age".into(), &int_type));
//
//         let store_type_op = StoreOp::StoreCustomType(type_name.clone(), type_fields);
//         let type_ref = vm.execute_store(store_type_op).unwrap();
//         let type_ref = type_ref.get(0).unwrap();
//
//         let name = vm.execute_store(StoreOp::StoreString("John".into())).unwrap();
//         let name = name.get(0).unwrap();
//
//         let age = vm.execute_store(StoreOp::StoreInt(25)).unwrap();
//         let age = age.get(0).unwrap();
//
//         let result = vm.execute_init(&type_ref, vec![&name, &age]);
//         assert!(result.is_ok());
//
//         let age2 = vm.execute_store(StoreOp::StoreString("test".to_string())).unwrap();
//         let age2 = age2.get(0).unwrap();
//
//         let result = vm.execute_init(&type_ref, vec![&name, &age2]);
//         assert!(result.is_err());
//
//     }
// }