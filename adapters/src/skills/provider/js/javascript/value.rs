// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, JsNativeError, JsResult, JsString, JsValue,
    object::{JsObject, builtins::JsArray},
    property::PropertyKey,
};
use std::collections::HashMap;

use app_domains::skills::models::CallValue;

pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Map(HashMap<String, Value>),
    List(Vec<Value>),
    Empty,
}

impl Value {
    pub fn new_from_call_value(value: &CallValue) -> Value {
        match value {
            CallValue::String(s) => Value::String(s.clone()),
            CallValue::Integer(i) => Value::Integer(*i),
            CallValue::Float(f) => Value::Float(*f),
            CallValue::Boolean(b) => Value::Bool(*b),
            CallValue::Array(a) => {
                let items = a.iter().map(|v| Value::new_from_call_value(v)).collect();
                Value::List(items)
            }
            CallValue::Object(o) => {
                let map = o
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::new_from_call_value(v)))
                    .collect();
                Value::Map(map)
            }
        }
    }

    pub fn new_from_js_value(value: &JsValue, context: &mut Context) -> JsResult<Value> {
        if value.is_undefined() || value.is_null() {
            return Ok(Value::Empty);
        }

        if let Some(b) = value.as_boolean() {
            return Ok(Value::Bool(b));
        }

        if let Some(n) = value.as_number() {
            if !n.is_finite() {
                return Err(JsNativeError::typ()
                    .with_message("non-finite number")
                    .into());
            }
            if n.fract() == 0.0 {
                return Ok(Value::Integer(n as i64));
            }
            return Ok(Value::Float(n));
        }

        if let Some(s) = value.as_string() {
            return Ok(Value::String(s.to_std_string_escaped()));
        }

        if let Some(obj) = value.as_object() {
            if obj.is_array() {
                let array = JsArray::from_object(obj.clone())?; // ? — already JsResult
                let len = array.length(context)?;
                let mut items = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let elem = array.get(i, context)?;
                    items.push(Value::new_from_js_value(&elem, context)?); // recurse
                }
                return Ok(Value::List(items));
            }

            let keys = obj.own_property_keys(context)?;
            let mut map = HashMap::new();
            for key in keys {
                let key_str = match &key {
                    PropertyKey::String(s) => s.to_std_string_escaped(),
                    PropertyKey::Index(i) => i.get().to_string(),
                    PropertyKey::Symbol(_) => continue,
                };
                let val = obj.get(key.clone(), context)?;
                map.insert(key_str, Value::new_from_js_value(&val, context)?);
            }
            return Ok(Value::Map(map));
        }

        Err(JsNativeError::typ()
            .with_message("unsupported type for conversion")
            .into())
    }

    pub fn into_js(self, context: &mut Context) -> JsResult<JsValue> {
        match self {
            Value::String(s) => Ok(JsValue::from(JsString::from(s))),
            Value::Integer(i) => Ok(JsValue::from(i)),
            Value::Float(f) => Ok(JsValue::from(f)),
            Value::Bool(b) => Ok(JsValue::from(b)),
            Value::Empty => Ok(JsValue::undefined()),

            Value::List(items) => {
                let converted: Vec<JsValue> = items
                    .into_iter()
                    .map(|v| v.into_js(context))
                    .collect::<JsResult<Vec<_>>>()?;
                let array = JsArray::from_iter(converted, context);
                Ok(array.into())
            }

            Value::Map(map) => {
                let obj = JsObject::with_object_proto(context.intrinsics());
                for (k, v) in map {
                    let jv = v.into_js(context)?;
                    obj.set(JsString::from(k), jv, false, context)?;
                }
                Ok(obj.into())
            }
        }
    }
}
