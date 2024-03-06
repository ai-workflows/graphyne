use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::data::live::{LiveData, ObjectLive, PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult};
use crate::runtime::static_state::state::StaticState;

pub fn execute_init(obj_type_ref: Arc<StoredData>, args: Vec<Arc<StoredData>>, static_state: Arc<StaticState>) -> ExecResult<Vec<PointerLive>> {
    let obj_type = obj_type_ref.stored_as_type()?;

    // make sure it is a custom type
    let obj_type = match obj_type {
        TypeLive::Custom(t_name, t_guid, t_fields) => (t_name, t_guid, t_fields),
        _ => return Err("Cannot execute operation init for non-custom type".to_string())
    };

    let (obj_t_name, _, obj_t_fields) = obj_type;

    if obj_t_fields.len() != args.len() {
        return Err(format!("Cannot initialize object of type {} with {} arguments, expected {}", obj_t_name, args.len(), obj_t_fields.len()));
    }

    let mut vals: HashMap<String, Arc<StoredData>> = HashMap::new();

    for (i, field) in obj_t_fields.iter().enumerate() {
        let (field_name, field_type_ptr) = field;

        // get the expected type of the field
        let field_type: &TypeLive = field_type_ptr.stored_as_type()?;

        let arg = &args[i];

        // do a type check if it isn't dynamic
        match field_type {
            TypeLive::Dynamic => (),
            _ => {
                let arg_type_ptr = match arg.as_ref().as_live().type_of(static_state.clone()) {
                    Some(Ok(ptr)) => ptr,
                    Some(Err(msg)) => return Err(format!("Could not get type of argument {}: {}", i, msg)),
                    None => return Err(format!("Cannot initialize object with argument {} of unknown type", i))
                };

                let arg_type: &TypeLive = arg_type_ptr.stored_as_type()?;

                if arg_type != field_type {
                    return Err(format!("Cannot initialize object of type {} with argument {} of type {}, expected {}", obj_t_name, i, arg_type.get_name(), field_type.get_name()));
                }
            }
        }

        vals.insert(field_name.clone(), arg.clone());
    }

    let stored_data = StoredData::ObjectStored(ObjectLive {
        type_ptr: obj_type_ref.clone(),
        fields: vals
    });

    Ok(vec![Arc::new(stored_data)])
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::runtime::data::live::{TypeLive, PointerLive};
//     use crate::runtime::data::stored::StoredData;
//     use crate::runtime::vm::mmu::store_op::StoreOp;
//     use crate::runtime::vm::value_ref::PointerLive;
//     use crate::runtime::vm::VM;
//     use std::intermediate::HashMap;
//     use crate::runtime::Symbol;
//
//     #[test]
//     fn test_execute_init() {
//         let vm = VM::new(2, 2);
//
//         let type_name: Symbol = "TestType".into();
//         let mut type_fields: Vec<(Symbol, Arc<StoredData>)> = vec![];
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