use std::collections::HashMap;
use crate::runtime::data::live::{PointerLive, TypeLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::data::stored::StoredData::DictStored;

pub fn jsonify(val: &StoredData,
) -> String {
    match val {
        StoredData::NullStored => "null".to_string(),
        StoredData::IntStored(val) => val.to_string(),
        StoredData::FloatStored(val) => val.to_string(),
        StoredData::StringStored(val) => val.clone(),
        StoredData::BoolStored(val) => val.to_string(),
        StoredData::PointerStored(ptr) => jsonify(ptr.as_ref()),
        StoredData::ListStored(list) => {
            let mut result = "[".to_string();
            for (i, item) in list.iter().enumerate() {
                let ptr_stored = StoredData::PointerStored(item.clone());

                result.push_str(&jsonify(&ptr_stored));

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
                map.insert(key.clone(), jsonify(&ptr_stored));
            }

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncV2Stored(val) => {
            let symbol_path = val.symbol_path.clone().join(".");
            symbol_path
        }
        StoredData::FuncStored(val) => {
            let mut map = HashMap::new();

            map.insert("input_vals".to_string(), jsonify(
                &StoredData::ListStored(val.input_vals.clone())));
            map.insert("output_vals".to_string(), jsonify(
                &StoredData::ListStored(val.output_vals.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncValStored(val) => {
            let mut map = HashMap::new();

            map.insert("guid".to_string(), val.guid.clone());
            map.insert("dependents".to_string(), jsonify(
                &StoredData::ListStored(val.dependents.clone())));
            if let Some(constant) = &val.constant {
                map.insert("constant".to_string(), jsonify(
                    &StoredData::PointerStored(constant.clone())));
            }
            map.insert("is_self".to_string(), jsonify(
                &StoredData::BoolStored(val.is_self)));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::FuncOpStored(val) => {
            let mut map = HashMap::new();

            map.insert("guid".to_string(), val.guid.clone());
            map.insert("opcode".to_string(), jsonify(
                &StoredData::IntStored(val.opcode as i64)));
            map.insert("input_vals".to_string(), jsonify(
                &StoredData::ListStored(val.input_vals.clone())));
            map.insert("output_vals".to_string(), jsonify(
                &StoredData::ListStored(val.output_vals.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::TypeStored(val) => {
            return match val {
                TypeLive::Custom(name, guid, fields) => {
                    let mut map = HashMap::new();

                    map.insert("name".to_string(), jsonify(
                        &StoredData::StringStored(name.clone())));
                    map.insert("guid".to_string(), jsonify(
                        &StoredData::StringStored(guid.clone())));

                    let mut fields_map: HashMap<String, PointerLive> = HashMap::new();

                    for (field_name, field_type_ptr) in fields {
                        fields_map.insert(field_name.clone(), field_type_ptr.clone());
                    }

                    map.insert("fields".to_string(), jsonify(
                        &DictStored(fields_map)));

                    serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
                },
                _ => val.get_name()
            };
        },
        StoredData::ObjectStored(val) => {
            let mut map = HashMap::new();

            map.insert("type".to_string(), jsonify(
                &StoredData::PointerStored(val.type_ptr.clone())));
            map.insert("data".to_string(), jsonify(
                &DictStored(val.fields.clone())));

            serde_json::to_string(&map).unwrap_or_else(|_| "null".to_string())
        }
        StoredData::StaticRefStored(val) => {
            match val.get() {
                Some(data) => jsonify(data),
                None => "null".to_string()
            }
        }
    }
}