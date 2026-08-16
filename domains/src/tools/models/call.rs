// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tools::models::{Schema, SchemaFieldType};

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

            match (field_schema.field_type.clone(), value) {
                (SchemaFieldType::String, CallValue::String(_)) => {}
                (SchemaFieldType::Integer, CallValue::Integer(_)) => {}
                (SchemaFieldType::Boolean, CallValue::Boolean(_)) => {}
                (SchemaFieldType::Number, CallValue::Float(_)) => {}
                (SchemaFieldType::Number, CallValue::Integer(_)) => {}
                (SchemaFieldType::Array, CallValue::Array(_)) => {}
                (SchemaFieldType::Object, CallValue::Object(_)) => {}
                _ => return false,
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
