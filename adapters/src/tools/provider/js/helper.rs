// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::HashMap;

use super::{
    CONFIGURATION_SKILL_CONFIGURATION, CONFIGURATION_SKILLS_DIR, JavascriptProvider, file,
    javascript::Value, models,
};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::{Call, ToolPermission},
};

impl JavascriptProvider {
    pub(super) async fn get_tools_path(&self, ctx: &AppContext) -> Result<String, AppError> {
        let path = ctx
            .configuration
            .get_string(ctx, CONFIGURATION_SKILLS_DIR)
            .await?;

        Ok(path)
    }

    pub(super) async fn load_package_info(
        &self,
        ctx: &AppContext,
        dir: &str,
    ) -> Result<models::Package, AppError> {
        let raw = file::read_file(ctx, &format!("{}/package.json", dir))?;
        let config: models::Package = serde_json::from_str(&raw).map_err(|e| {
            app_error!(
                Internal,
                "tool_load_failed",
                &format!("Failed to parse package.json in {}: {}", dir, e),
                ctx.clone()
            )
        })?;

        Ok(config)
    }

    pub(super) fn input_to_value(&self, input: &Call) -> Value {
        Value::Map(
            input
                .arguments
                .iter()
                .map(|(k, v)| (k.clone(), Value::new_from_call_value(v)))
                .collect::<HashMap<String, Value>>(),
        )
    }

    pub(super) fn permission_to_value(&self, permission: &ToolPermission) -> Value {
        Value::Map(HashMap::from_iter(vec![
            ("name".to_string(), Value::String(permission.name.clone())),
            (
                "scopes".to_string(),
                Value::List(
                    permission
                        .scopes
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            ),
        ]))
    }

    pub(super) async fn get_tool_configuration(
        &self,
        ctx: &AppContext,
        name: &str,
    ) -> Result<Value, AppError> {
        let tool_configuration = match ctx
            .configuration
            .get_map(
                ctx,
                &format!("{}.js_{}", CONFIGURATION_SKILL_CONFIGURATION, name),
            )
            .await
        {
            Ok(config) => config,
            Err(_) => HashMap::new(),
        };

        Ok(Value::Map(
            tool_configuration
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<HashMap<String, Value>>(),
        ))
    }
}
