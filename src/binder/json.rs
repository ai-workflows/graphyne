use serde_json::{Map, Number, Value};
use crate::runtime::data::live::TypeLive;
use crate::runtime::data::stored::StoredData;

fn to_json_value(val: &StoredData) -> Value {
    match val {
        StoredData::NullStored => Value::Null,
        StoredData::IntStored(val) => Value::Number(Number::from(*val)),
        StoredData::FloatStored(val) => Number::from_f64(*val)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        StoredData::StringStored(val) => Value::String(val.clone()),
        StoredData::BoolStored(val) => Value::Bool(*val),
        StoredData::PointerStored(ptr) => to_json_value(ptr.as_ref()),
        StoredData::ListStored(list) => Value::Array(
            list.iter()
                .map(|item| to_json_value(item.as_ref()))
                .collect()
        ),
        StoredData::DictStored(dict) => Value::Object(
            dict.iter()
                .map(|(key, val)| (key.clone(), to_json_value(val.as_ref())))
                .collect::<Map<String, Value>>()
        ),
        StoredData::FuncStored(val) => Value::String(val.symbol_path.join(".")),
        StoredData::TypeStored(val) => match val {
            TypeLive::Custom(name, guid, fields) => {
                let fields = fields.iter()
                    .map(|(field_name, field_type_ptr)| {
                        (field_name.clone(), to_json_value(field_type_ptr.as_ref()))
                    })
                    .collect::<Map<String, Value>>();

                Value::Object(Map::from_iter([
                    ("name".to_string(), Value::String(name.clone())),
                    ("guid".to_string(), Value::String(guid.clone())),
                    ("fields".to_string(), Value::Object(fields)),
                ]))
            }
            _ => Value::String(val.get_name())
        },
        StoredData::ObjectStored(val) => Value::Object(Map::from_iter([
            ("type".to_string(), to_json_value(val.type_ptr.as_ref())),
            (
                "data".to_string(),
                Value::Object(
                    val.fields.iter()
                        .map(|(key, value)| (key.clone(), to_json_value(value.as_ref())))
                        .collect::<Map<String, Value>>()
                )
            ),
        ])),
        StoredData::StaticRefStored(val) => match val.get() {
            Some(data) => to_json_value(data),
            None => Value::Null,
        }
    }
}

pub fn jsonify(val: &StoredData) -> String {
    serde_json::to_string(&to_json_value(val)).unwrap_or_else(|_| "null".to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use crate::binder::json::jsonify;
    use crate::runtime::data::live::{ObjectLive, TypeLive};
    use crate::runtime::data::stored::StoredData;

    #[test]
    fn stringify_string_values_as_valid_json() {
        assert_eq!(jsonify(&StoredData::StringStored("World".to_string())), "\"World\"");
    }

    #[test]
    fn stringify_nested_dict_values_without_double_encoding() {
        let mut dict = HashMap::new();
        dict.insert("message".to_string(), Arc::new(StoredData::StringStored("hello".to_string())));
        dict.insert("count".to_string(), Arc::new(StoredData::IntStored(2)));

        assert_eq!(jsonify(&StoredData::DictStored(dict)), r#"{"count":2,"message":"hello"}"#);
    }

    #[test]
    fn stringify_objects_as_nested_json() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), Arc::new(StoredData::StringStored("Ada".to_string())));

        let obj = ObjectLive {
            type_ptr: Arc::new(StoredData::TypeStored(TypeLive::String)),
            fields,
        };

        assert_eq!(
            jsonify(&StoredData::ObjectStored(obj)),
            r#"{"data":{"name":"Ada"},"type":"String"}"#
        );
    }

    #[test]
    fn stringify_custom_types_as_structured_json() {
        let custom_type = TypeLive::Custom(
            "Person".to_string(),
            "guid-1".to_string(),
            vec![(
                "name".to_string(),
                Arc::new(StoredData::TypeStored(TypeLive::String)),
            )],
        );

        assert_eq!(
            jsonify(&StoredData::TypeStored(custom_type)),
            r#"{"fields":{"name":"String"},"guid":"guid-1","name":"Person"}"#
        );
    }
}
