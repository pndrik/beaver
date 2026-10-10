// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::Configuration,
    },
};

/// Configuration backed by a fixed map of dotted keys to JSON values, e.g.
/// `"flows.path" => "/flows"`.
#[derive(Debug, Default, Clone)]
pub struct ConfigurationMap {
    values: HashMap<String, Value>,
}

impl ConfigurationMap {
    pub fn new(values: HashMap<String, Value>) -> Self {
        Self { values }
    }

    fn get(&self, ctx: &AppContext, key: &str) -> Result<&Value, AppError> {
        self.values.get(key).ok_or_else(|| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Configuration key '{}' not found", key),
                ctx.clone()
            )
        })
    }

    fn invalid_type(ctx: &AppContext, key: &str, expected: &str) -> AppError {
        app_error!(
            Internal,
            "configuration_load_failed",
            &format!("Configuration key '{}' is not a {}", key, expected),
            ctx.clone()
        )
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for ConfigurationMap {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::new(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }
}

#[async_trait]
impl Configuration for ConfigurationMap {
    async fn get_string(&self, ctx: &AppContext, key: &str) -> Result<String, AppError> {
        self.get(ctx, key)?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Self::invalid_type(ctx, key, "string"))
    }

    async fn get_int(&self, ctx: &AppContext, key: &str) -> Result<i64, AppError> {
        self.get(ctx, key)?
            .as_i64()
            .ok_or_else(|| Self::invalid_type(ctx, key, "integer"))
    }

    async fn get_bool(&self, ctx: &AppContext, key: &str) -> Result<bool, AppError> {
        self.get(ctx, key)?
            .as_bool()
            .ok_or_else(|| Self::invalid_type(ctx, key, "boolean"))
    }

    async fn get_map(
        &self,
        ctx: &AppContext,
        key: &str,
    ) -> Result<HashMap<String, String>, AppError> {
        self.get(ctx, key)?
            .as_object()
            .and_then(|object| {
                object
                    .iter()
                    .map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                    .collect()
            })
            .ok_or_else(|| Self::invalid_type(ctx, key, "map of strings"))
    }

    async fn get_json_value(&self, ctx: &AppContext, key: &str) -> Result<Value, AppError> {
        self.get(ctx, key).cloned()
    }
}
