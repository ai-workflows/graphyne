use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::data::live::{PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::data::stored::StoredData::DictStored;
use crate::runtime::mmu::mmu::{MMU, value_ref_from_ptr};

pub fn jsonify(mmu: Arc<MMU>,
               val: &StoredData,
) -> String {
    match val {
        StoredData::NullStored => "null".to_string(),
        StoredData::IntStored(val) => val.to_string(),
        StoredData::FloatStored(val) => val.to_string(),
        StoredData::StringStored(val) => val.clone(),
        StoredData::BoolStored(val) => val.to_string(),
        StoredData::PointerStored(ptr) => {
            let val_ref = match value_ref_from_ptr(mmu.clone(), ptr.clone()) {
                Ok(val_ref) => val_ref,
                Err(_) => return "null".to_string(),
            };
            match mmu.get_ref_value(&val_ref) {
                Ok(val) => jsonify(mmu.clone(), &val),
                Err(_) => return "null".to_string(),
            }
        }
        StoredData::ListStored(list) => {
            let mut result = "[".to_string();
            for (i, item) in list.iter().enumerate() {
                let ptr_stored = StoredData::PointerStored(item.clone());

                result.push_str(&jsonify(mmu.clone(), &ptr_stored));

                if i < list.len() - 1 {
                    result.push_str(", ");
                }
            }
            result.push_str("]");
            result
        }
        DictStored(dict) => {
            let mut map = HashMap::new();

            for (key, val) in dict {
                let ptr_stored = StoredData::PointerStored(val.clone());
                map.insert(key.clone(), jsonify(mmu.clone(), &ptr_stored));
            }

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncStored(val) => {
            let mut map = HashMap::new();

            map.insert("input_vals".to_string(), jsonify(
                mmu.clone(),
                &StoredData::ListStored(val.input_vals.clone())));
            map.insert("output_vals".to_string(), jsonify(
                mmu.clone(),
                &StoredData::ListStored(val.output_vals.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncValStored(val) => {
            let mut map = HashMap::new();

            map.insert("guid".to_string(), val.guid.clone());
            map.insert("dependents".to_string(), jsonify(
                mmu.clone(),
                &StoredData::ListStored(val.dependents.clone())));
            if let Some(constant) = &val.constant {
                map.insert("constant".to_string(), jsonify(
                    mmu.clone(),
                    &StoredData::PointerStored(constant.clone())));
            }
            map.insert("is_self".to_string(), jsonify(
                mmu.clone(),
                &StoredData::BoolStored(val.is_self)));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncOpStored(val) => {
            let mut map = HashMap::new();

            map.insert("guid".to_string(), val.guid.clone());
            map.insert("opcode".to_string(), jsonify(
                mmu.clone(),
                &StoredData::IntStored(val.opcode as i64)));
            map.insert("input_vals".to_string(), jsonify(
                mmu.clone(),
                &StoredData::ListStored(val.input_vals.clone())));
            map.insert("output_vals".to_string(), jsonify(
                mmu.clone(),
                &StoredData::ListStored(val.output_vals.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::TypeStored(val) => {
            return match val {
                TypeLive::Custom(name, guid, fields) => {
                    let mut map = HashMap::new();

                    map.insert("name".to_string(), jsonify(
                        mmu.clone(),
                        &StoredData::StringStored(name.clone())));
                    map.insert("guid".to_string(), jsonify(
                        mmu.clone(),
                        &StoredData::StringStored(guid.clone())));

                    let mut fields_map: HashMap<String, PointerLive> = HashMap::new();

                    for (field_name, field_type_ptr) in fields {
                        fields_map.insert(field_name.clone(), field_type_ptr.clone());
                    }

                    map.insert("fields".to_string(), jsonify(
                        mmu.clone(),
                        &DictStored(fields_map)));

                    serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
                },
                _ => val.get_name()
            };
        },
        StoredData::ObjectStored(val) => {
            let mut map = HashMap::new();

            map.insert("type".to_string(), jsonify(
                mmu.clone(),
                &StoredData::PointerStored(val.type_ptr.clone())));
            map.insert("data".to_string(), jsonify(
                mmu.clone(),
                &DictStored(val.fields.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())}
    }
}