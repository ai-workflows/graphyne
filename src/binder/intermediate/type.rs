use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{SeqAccess};
use crate::runtime::data::live::TypeLive;
use crate::runtime::{Symbol};

#[derive(Debug, Clone, Serialize)]
pub enum CollectionType {
    Any,
    Null,
    Int,
    Float,
    Str,
    Bool,
    List,
    Dict,
    Type,
    Custom(Symbol)
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomTypeDef(pub Vec<(Symbol, CollectionTypeConst)>);

#[derive(Debug, Clone, Serialize)]
pub struct CollectionTypeConst(pub CollectionType);

impl From<TypeLive> for CollectionTypeConst {
    fn from(value: TypeLive) -> Self {
        match value {
            TypeLive::Integer => CollectionTypeConst(CollectionType::Int),
            TypeLive::Float => CollectionTypeConst(CollectionType::Float),
            TypeLive::String => CollectionTypeConst(CollectionType::Str),
            TypeLive::Boolean => CollectionTypeConst(CollectionType::Bool),
            TypeLive::Pointer => CollectionTypeConst(CollectionType::Int),
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
                    "list" => Ok(CollectionTypeConst(CollectionType::List)),
                    "dict" => Ok(CollectionTypeConst(CollectionType::Dict)),
                    "type" => Ok(CollectionTypeConst(CollectionType::Type)),
                    _ => Ok(CollectionTypeConst(CollectionType::Custom(value.into())))
                }
            }
        }

        deserializer.deserialize_str(CTypeVisitor)
    }
}

// deserialize a list of fields into a custom type definition
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

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
                let mut fields: Vec<(Symbol, CollectionTypeConst)> = Vec::new();

                while let Some(field) = seq.next_element()? {
                    fields.push(field);
                }
                Ok(CustomTypeDef(fields))
            }
        }

        deserializer.deserialize_seq(CTypeVisitor)
    }
}