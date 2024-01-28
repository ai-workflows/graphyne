use std::collections::HashMap;
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::live::PointerLive;
use crate::core::ExecResult;

pub fn type_of_helper(target_type: &TypeLive, type_map: &HashMap<TypeLive, usize>) -> Option<ExecResult<PointerLive>> {
    // get the type pointer from the type map
    let type_ptr_id = match type_map.get(&target_type) {
        Some(type_ptr) => type_ptr,
        None => return Some(Err(format!("Failed to get pointer to type {:?}", target_type))),
    };

    // return the pointer
    Some(Ok(PointerLive {
        id: *type_ptr_id,
        phantom: Default::default(),
        counted: false,
    }))
}