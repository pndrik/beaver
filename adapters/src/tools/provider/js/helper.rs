// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use super::{
    CONFIGURATION_SKILL_CONFIGURATION, CONFIGURATION_SKILLS_DIR, JavascriptProvider, file, models,
};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::ToolPermission,
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

    pub(super) fn permission_to_value(&self, permission: &ToolPermission) -> Value {
        json!({ "name": permission.name, "scopes": permission.scopes })
    }

    pub(super) async fn get_tool_configuration(
        &self,
        ctx: &AppContext,
        name: &str,
    ) -> Result<Value, AppError> {
        let tool_configuration = ctx
            .configuration
            .get_map(
                ctx,
                &format!("{}.js_{}", CONFIGURATION_SKILL_CONFIGURATION, name),
            )
            .await
            .unwrap_or_default();

        Ok(json!(tool_configuration))
    }
}
