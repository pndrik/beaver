// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tools::models::{Schema, SchemaField, SchemaFieldType};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Call {
    pub name: String,
    pub arguments: HashMap<String, CallValue>,
}
impl Call {
    pub fn new(name: String, arguments: HashMap<String, CallValue>) -> Self {
        Self { name, arguments }
    }

    pub fn get_argument(&self, key: &str) -> Option<&CallValue> {
        self.arguments.get(key)
    }

    pub fn arguments_as_json(&self) -> String {
        serde_json::to_string(&self.arguments).unwrap_or_default()
    }

    pub fn arguments_as_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::to_value(&self.arguments)
            .unwrap_or_default()
            .as_object()
            .cloned()
            .unwrap_or_default()
    }

    pub fn arguments_into<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::to_value(&self.arguments).and_then(serde_json::from_value)
    }

    pub fn validate_arguments(&self, schema: Schema) -> bool {
        for field in schema.required {
            if !self.arguments.contains_key(&field) {
                return false;
            }
        }

        for (key, value) in &self.arguments {
            let Some(field_schema) = schema.properties.get(key) else {
                return false;
            };

            if !field_schema.allows(value) {
                return false;
            }
        }

        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CallValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),

    Array(Vec<CallValue>),
    Object(HashMap<String, CallValue>),

    Null,
}
impl CallValue {
    pub fn as_string(&self) -> Option<String> {
        if let CallValue::String(s) = self {
            Some(s.clone())
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let CallValue::Integer(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let CallValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

impl SchemaField {
    /// Whether `value` has one of the allowed types and, if the field has an
    /// `enum`, is one of its values. Integers also count as numbers.
    pub fn allows(&self, value: &CallValue) -> bool {
        let type_matches = match value {
            CallValue::String(_) => self.field_type.allows(&SchemaFieldType::String),
            CallValue::Integer(_) => {
                self.field_type.allows(&SchemaFieldType::Integer)
                    || self.field_type.allows(&SchemaFieldType::Number)
            }
            CallValue::Float(_) => self.field_type.allows(&SchemaFieldType::Number),
            CallValue::Boolean(_) => self.field_type.allows(&SchemaFieldType::Boolean),
            CallValue::Array(_) => self.field_type.allows(&SchemaFieldType::Array),
            CallValue::Object(_) => self.field_type.allows(&SchemaFieldType::Object),
            CallValue::Null => self.field_type.allows(&SchemaFieldType::Null),
        };

        type_matches
            && self.enumeration.as_ref().is_none_or(|allowed| {
                let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
                allowed
                    .iter()
                    .any(|candidate| json_equal(candidate, &value))
            })
    }
}

/// JSON equality that compares numbers by value, so `1` equals `1.0`.
fn json_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) if a.is_number() && b.is_number() => x == y,
        _ => a == b,
    }
}
