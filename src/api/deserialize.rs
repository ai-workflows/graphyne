use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use crate::api::collections::c_const::{CCData, CollectionConst};
use crate::api::collections::func::CFnValueNode;
use crate::api::functions::FunctionOpNode;
use crate::core::data::functions::OpCode;
use crate::core::Symbol;

impl<'de> Deserialize<'de> for FunctionOpNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        // can be a tuple of (opcode, input_vals, output_vals) or a map of {opcode, input_vals, output_vals}
        struct FunctionOpNodeVisitor;

        impl<'de> serde::de::Visitor<'de> for FunctionOpNodeVisitor {
            type Value = FunctionOpNode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of (opcode, input_vals, output_vals) or a map of {opcode, input_vals, output_vals}")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
                let opcode: OpCode = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let input_vals: Vec<Symbol> = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                // let output_vals: Vec<Symbol> = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                // if there is only one output, it may be a single symbol instead of a list
                let output_vals: Vec<Symbol> = match seq.next_element()? {
                    Some(val) => vec![val],
                    None => match seq.next_element()? {
                        Some(val) => val,
                        None => return Err(serde::de::Error::invalid_length(2, &self)),
                    }
                };

                Ok(FunctionOpNode {
                    opcode,
                    input_vals,
                    output_vals,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de>, {
                let mut opcode: Option<OpCode> = None;
                let mut input_vals: Option<Vec<Symbol>> = None;
                let mut output_vals: Option<Vec<Symbol>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "opcode" => {
                            if opcode.is_some() {
                                return Err(serde::de::Error::duplicate_field("opcode"));
                            }

                            opcode = Some(map.next_value()?);
                        },
                        "input_vals" => {
                            if input_vals.is_some() {
                                return Err(serde::de::Error::duplicate_field("input_vals"));
                            }

                            input_vals = Some(map.next_value()?);
                        },
                        "output_vals" => {
                            if output_vals.is_some() {
                                return Err(serde::de::Error::duplicate_field("output_vals"));
                            }

                            output_vals = Some(match map.next_value()? {
                                Some(val) => vec![val],
                                None => match map.next_value()? {
                                    Some(val) => val,
                                    None => return Err(serde::de::Error::invalid_length(2, &self)),
                                }
                            });
                        },
                        _ => {
                            return Err(serde::de::Error::unknown_field(key, &["opcode", "input_vals", "output_vals"]));
                        }
                    }
                }

                let opcode = opcode.ok_or_else(|| serde::de::Error::missing_field("opcode"))?;
                let input_vals = input_vals.ok_or_else(|| serde::de::Error::missing_field("input_vals"))?;
                let output_vals = output_vals.ok_or_else(|| serde::de::Error::missing_field("output_vals"))?;

                Ok(FunctionOpNode {
                    opcode,
                    input_vals,
                    output_vals,
                })
            }
        }

        deserializer.deserialize_any(FunctionOpNodeVisitor)
    }
}

impl<'de> Deserialize<'de> for CCData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct CCDataVisitor;

        impl<'de> serde::de::Visitor<'de> for CCDataVisitor {
            type Value = CCData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CCData")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(CCData::Bool(value.into()))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(CCData::Int(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(CCData::Int(value as i64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> where E: serde::de::Error {
                // check if the float is an integer
                if value.fract() == 0.0 {
                    Ok(CCData::Int(value as i64))
                } else {
                    Ok(CCData::Float(value.into()))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(CCData::String(value.into()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de> {
                let mut list = Vec::new();

                while let Some(elem) = seq.next_element()? {
                    list.push(elem);
                }

                Ok(CCData::List(list))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de> {
                let mut dict = HashMap::new();

                while let Some((key, value)) = map.next_entry()? {
                    dict.insert(key, value);
                }

                Ok(CCData::Dict(dict))
            }
        }

        deserializer.deserialize_any(CCDataVisitor)
    }
}

impl<'de> Deserialize<'de> for CollectionConst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        Ok(CollectionConst(CCData::deserialize(deserializer)?))
    }
}

impl<'de> Deserialize<'de> for CFnValueNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct CFnValueNodeVisitor;

        impl<'de> serde::de::Visitor<'de> for CFnValueNodeVisitor {
            type Value = CFnValueNode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of (symbol, constant) or a map of {symbol, constant}")
            }

            // if it a string, that means it is a variable
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(CFnValueNode {
                    symbol: value.into(),
                    constant: None,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
                // if there is no constant, the tuple will only have one element
                let symbol: Symbol = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let constant: Option<CCData> = seq.next_element()?;

                Ok(CFnValueNode {
                    symbol,
                    constant,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de>, {
                let mut symbol: Option<Symbol> = None;
                let mut constant: Option<CCData> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "symbol" => {
                            if symbol.is_some() {
                                return Err(serde::de::Error::duplicate_field("symbol"));
                            }

                            symbol = Some(map.next_value()?);
                        },
                        "constant" => {
                            if constant.is_some() {
                                return Err(serde::de::Error::duplicate_field("constant"));
                            }

                            constant = Some(map.next_value()?);
                        },
                        _ => {
                            return Err(serde::de::Error::unknown_field(key, &["symbol", "constant"]));
                        }
                    }
                }

                let symbol = symbol.ok_or_else(|| serde::de::Error::missing_field("symbol"))?;

                Ok(CFnValueNode {
                    symbol,
                    constant,
                })
            }
        }

        deserializer.deserialize_any(CFnValueNodeVisitor)

    }
}