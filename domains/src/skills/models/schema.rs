// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::app_error;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SchemaFieldType {
    String,
    Integer,
    Number,
    Boolean,
    Null,

    Array,
    #[default]
    Object,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SchemaItems {
    Single(Box<SchemaField>),
    Tuple(Vec<SchemaField>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaField {
    #[serde(rename = "type", default)]
    pub field_type: SchemaFieldType,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enumeration: Option<Vec<String>>,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<SchemaItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, SchemaField>>,
}

impl SchemaField {
    pub fn new(
        field_type: SchemaFieldType,
        description: &str,
        enumeration: Option<Vec<String>>,
    ) -> Self {
        Self {
            field_type,
            enumeration,
            description: description.to_string(),
            required: None,
            items: None,
            properties: None,
        }
    }

    pub fn add_child(
        &mut self,
        ctx: &AppContext,
        name: &str,
        required: bool,
        field: SchemaField,
    ) -> Result<(), AppError> {
        match self.field_type {
            SchemaFieldType::Object => self.add_property(ctx, name, required, field),
            SchemaFieldType::Array => self.add_item(ctx, field),
            _ => Err(app_error!(
                Validation,
                "internal_error",
                format!("Cannot add child to field of type {:?}", self.field_type),
                ctx.clone()
            )),
        }
    }

    pub fn set_items(&mut self, ctx: &AppContext, field: SchemaField) -> Result<(), AppError> {
        if self.field_type != SchemaFieldType::Array {
            return Err(app_error!(
                Validation,
                "internal_error",
                format!("Cannot add items to field of type {:?}", self.field_type),
                ctx.clone()
            ));
        }
        self.items = Some(SchemaItems::Single(Box::new(field)));
        Ok(())
    }

    pub fn add_item(&mut self, ctx: &AppContext, field: SchemaField) -> Result<(), AppError> {
        if self.field_type != SchemaFieldType::Array {
            return Err(app_error!(
                Validation,
                "internal_error",
                format!("Cannot add child to field of type {:?}", self.field_type),
                ctx.clone()
            ));
        }
        match &mut self.items {
            Some(SchemaItems::Tuple(v)) => v.push(field),
            None => self.items = Some(SchemaItems::Tuple(vec![field])),
            Some(SchemaItems::Single(_)) => {
                return Err(app_error!(
                    Validation,
                    "internal_error",
                    "Cannot add a tuple item to an array whose items is a single schema"
                        .to_string(),
                    ctx.clone()
                ));
            }
        }
        Ok(())
    }

    pub fn add_property(
        &mut self,
        ctx: &AppContext,
        name: &str,
        required: bool,
        field: SchemaField,
    ) -> Result<(), AppError> {
        if self.field_type != SchemaFieldType::Object {
            return Err(app_error!(
                Validation,
                "internal_error",
                format!("Cannot add child to field of type {:?}", self.field_type),
                ctx.clone()
            ));
        }
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        if required {
            if self.required.is_none() {
                self.required = Some(Vec::new());
            }
            self.required.as_mut().unwrap().push(name.to_string());
        }
        self.properties
            .as_mut()
            .unwrap()
            .insert(name.to_string(), field);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schema {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub field_type: SchemaFieldType,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub properties: HashMap<String, SchemaField>,
}

impl Schema {
    pub fn new(title: &str, description: &str) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            field_type: SchemaFieldType::Object,
            required: Vec::new(),
            properties: HashMap::new(),
        }
    }

    pub fn add_property(
        &mut self,
        name: &str,
        required: bool,
        field: SchemaField,
    ) -> Result<(), AppError> {
        if required {
            self.required.push(name.to_string());
        }
        self.properties.insert(name.to_string(), field);
        Ok(())
    }

    pub fn to_json_value(&self, ctx: &AppContext) -> Result<serde_json::Value, AppError> {
        serde_json::to_value(self).map_err(|e| {
            app_error!(
                Internal,
                "internal_error",
                format!("Failed to serialize schema: {}", e),
                ctx.clone()
            )
        })
    }

    pub fn from_json_input_schema(
        ctx: &AppContext,
        map: serde_json::Map<String, serde_json::Value>,
    ) -> Result<Self, AppError> {
        serde_json::from_value(serde_json::Value::Object(map)).map_err(|e| {
            app_error!(
                Validation,
                "invalid_schema",
                format!("Failed to deserialize input schema: {}", e),
                ctx.clone()
            )
        })
    }
}
