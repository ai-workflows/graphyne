use std::collections::HashMap;
use crate::runtime::data::live::live_data::TypeLive;
use crate::runtime::data::live::PointerLive;
use crate::runtime::ExecResult;

pub fn type_of_helper(target_type: &TypeLive, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
    // get the type pointer from the type map
    let type_ptr_id = match type_map.get(&target_type) {
        Some(ptr) => ptr.id,
        None => return Some(Err(format!("Failed to get pointer to type {:?}", target_type))),
    };

    // return the pointer
    Some(Ok(PointerLive {
        id: type_ptr_id.clone(),
        phantom: Default::default(),
        counted: false,
    }))
}