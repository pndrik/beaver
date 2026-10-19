// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    #[default]
    String,
    Number,
    Float,
}

impl ValueType {
    pub fn parse(&self, value: &Value) -> Option<Value> {
        match (self, value) {
            (ValueType::String, Value::String(_)) => Some(value.clone()),
            (ValueType::String, Value::Number(_) | Value::Bool(_)) => {
                Some(Value::String(value.to_string()))
            }
            (ValueType::Number, Value::Number(number)) => number.as_i64().map(Value::from),
            (ValueType::Number, Value::String(text)) => {
                text.trim().parse::<i64>().ok().map(Value::from)
            }
            (ValueType::Float, Value::Number(number)) => number.as_f64().map(Value::from),
            (ValueType::Float, Value::String(text)) => text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|float| float.is_finite())
                .map(Value::from),
            _ => None,
        }
    }

    pub fn zero(&self) -> Value {
        match self {
            ValueType::String => Value::from(""),
            ValueType::Number => Value::from(0),
            ValueType::Float => Value::from(0.0),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ValueType::String => "string",
            ValueType::Number => "number",
            ValueType::Float => "float",
        }
    }
}
