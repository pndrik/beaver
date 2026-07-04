// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::ConfigurationUniversal;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

impl ConfigurationUniversal {
    pub(super) fn get_direct_string(
        &self,
        ctx: &AppContext,
        key: &str,
    ) -> Result<String, AppError> {
        let config = self.get_config(ctx)?;

        config.get_string(key).map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to get configuration value: {}", e),
                ctx.clone()
            )
        })
    }

    pub(super) fn get_direct_int(&self, ctx: &AppContext, key: &str) -> Result<i64, AppError> {
        let config = self.get_config(ctx)?;

        config.get_int(key).map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to get configuration value: {}", e),
                ctx.clone()
            )
        })
    }

    pub(super) fn get_direct_bool(&self, ctx: &AppContext, key: &str) -> Result<bool, AppError> {
        let config = self.get_config(ctx)?;

        config.get_bool(key).map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to get configuration value: {}", e),
                ctx.clone()
            )
        })
    }
}
