// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::app_error;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaFieldType {
    String,
    Integer,
    Boolean,
    Null,

    Array,
    Object,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaField {
    #[serde(rename = "type")]
    pub field_type: SchemaFieldType,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enumeration: Option<Vec<String>>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SchemaField>>,
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

    pub fn add_item(&mut self, ctx: &AppContext, field: SchemaField) -> Result<(), AppError> {
        if self.field_type != SchemaFieldType::Array {
            return Err(app_error!(
                Validation,
                "internal_error",
                format!("Cannot add child to field of type {:?}", self.field_type),
                ctx.clone()
            ));
        }
        if self.items.is_none() {
            self.items = Some(Vec::new());
        }
        self.items.as_mut().unwrap().push(field);
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
}
