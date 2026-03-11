use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{MapAccess, SeqAccess};
use crate::runtime::data::live::TypeLive;
use crate::runtime::{Symbol, SymbolPath};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum CollectionType {
    Any,
    Null,
    Int,
    Float,
    Str,
    Bool,
    Pointer,
    List,
    Dict,
    Type,
    Custom(SymbolPath)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CustomTypeDef(pub Vec<(Symbol, CollectionTypeConst)>);

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CollectionTypeConst(pub CollectionType);

impl From<TypeLive> for CollectionTypeConst {
    fn from(value: TypeLive) -> Self {
        match value {
            TypeLive::Integer => CollectionTypeConst(CollectionType::Int),
            TypeLive::Float => CollectionTypeConst(CollectionType::Float),
            TypeLive::String => CollectionTypeConst(CollectionType::Str),
            TypeLive::Boolean => CollectionTypeConst(CollectionType::Bool),
            TypeLive::Pointer => CollectionTypeConst(CollectionType::Pointer),
            TypeLive::List => CollectionTypeConst(CollectionType::List),
            TypeLive::Dictionary => CollectionTypeConst(CollectionType::Dict),
            TypeLive::Null => CollectionTypeConst(CollectionType::Null),
            TypeLive::Type => CollectionTypeConst(CollectionType::Type),
            TypeLive::Dynamic => CollectionTypeConst(CollectionType::Any),
            _ => panic!("Cannot convert type to collection type")
        }
    }
}

impl<'de> Deserialize<'de> for CollectionTypeConst {
    fn deserialize<D>(deserializer: D) -> Result<CollectionTypeConst, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CTypeVisitor;

        impl<'de> serde::de::Visitor<'de> for CTypeVisitor {
            type Value = CollectionTypeConst;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a collection type")
            }

            fn visit_str<E>(self, value: &str) -> Result<CollectionTypeConst, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "any" => Ok(CollectionTypeConst(CollectionType::Any)),
                    "null" => Ok(CollectionTypeConst(CollectionType::Null)),
                    "int" => Ok(CollectionTypeConst(CollectionType::Int)),
                    "float" => Ok(CollectionTypeConst(CollectionType::Float)),
                    "str" => Ok(CollectionTypeConst(CollectionType::Str)),
                    "bool" => Ok(CollectionTypeConst(CollectionType::Bool)),
                    "pointer" => Ok(CollectionTypeConst(CollectionType::Pointer)),
                    "list" => Ok(CollectionTypeConst(CollectionType::List)),
                    "dict" => Ok(CollectionTypeConst(CollectionType::Dict)),
                    "type" => Ok(CollectionTypeConst(CollectionType::Type)),
                    _ => {
                        let parts: Vec<Symbol> = value.split('.')
                            .map(|s| s.to_string())
                            .collect();
                        Ok(CollectionTypeConst(CollectionType::Custom(parts)))
                    }
                }
            }
        }

        deserializer.deserialize_str(CTypeVisitor)
    }
}

// deserialize a list or map of fields into a custom type definition
impl<'de> Deserialize<'de> for CustomTypeDef {
    fn deserialize<D>(deserializer: D) -> Result<CustomTypeDef, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CTypeVisitor;

        impl<'de> serde::de::Visitor<'de> for CTypeVisitor {
            type Value = CustomTypeDef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a custom type definition")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut fields: Vec<(Symbol, CollectionTypeConst)> = Vec::new();

                while let Some(field) = seq.next_element()? {
                    fields.push(field);
                }
                Ok(CustomTypeDef(fields))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut fields: Vec<(Symbol, CollectionTypeConst)> = Vec::new();

                while let Some((field_name, field_type)) = map.next_entry()? {
                    fields.push((field_name, field_type));
                }

                Ok(CustomTypeDef(fields))
            }
        }

        deserializer.deserialize_any(CTypeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::binder::intermediate::r#type::{CollectionType, CollectionTypeConst, CustomTypeDef};
    use crate::runtime::data::live::TypeLive;

    #[test]
    fn pointer_type_maps_to_pointer_collection_type() {
        assert_eq!(CollectionTypeConst::from(TypeLive::Pointer), CollectionTypeConst(CollectionType::Pointer));
    }

    #[test]
    fn deserialize_pointer_collection_type() {
        let parsed: CollectionTypeConst = serde_json::from_str("\"pointer\"").unwrap();
        assert_eq!(parsed, CollectionTypeConst(CollectionType::Pointer));
    }

    #[test]
    fn deserialize_custom_type_def_from_map_form() {
        let parsed: CustomTypeDef = serde_json::from_str(r#"{
            "name": "str",
            "age": "int"
        }"#).unwrap();

        assert_eq!(
            parsed,
            CustomTypeDef(vec![
                ("name".to_string(), CollectionTypeConst(CollectionType::Str)),
                ("age".to_string(), CollectionTypeConst(CollectionType::Int)),
            ])
        );
    }
}
