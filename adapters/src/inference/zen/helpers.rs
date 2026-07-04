// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{CONFIGURATION_APIKEY, CONFIGURATION_ENDPOINT, Zen};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

impl Zen {
    async fn get_endpoint(&self, ctx: &AppContext) -> Result<String, AppError> {
        ctx.configuration
            .get_string(ctx, CONFIGURATION_ENDPOINT)
            .await
    }

    async fn get_apikey(&self, ctx: &AppContext) -> Result<String, AppError> {
        ctx.configuration
            .get_string(ctx, CONFIGURATION_APIKEY)
            .await
    }

    pub(super) async fn list_available_models(
        &self,
        ctx: &AppContext,
    ) -> Result<Vec<String>, AppError> {
        let endpoint = self.get_endpoint(ctx).await?;
        let api_key = self.get_apikey(ctx).await?;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/models", endpoint.trim_end_matches("/")))
            .bearer_auth(&api_key)
            .send()
            .await
            .map_err(|e| {
                app_error!(
                    Internal,
                    "inference_failed",
                    &format!("Failed to list available models: {}", e),
                    ctx.clone()
                )
            })?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.map_err(|e| {
            app_error!(
                Internal,
                "inference_failed",
                &format!("Failed to parse response body: {}", e),
                ctx.clone()
            )
        })?;

        if !status.is_success() {
            return Err(app_error!(
                Internal,
                "inference_failed",
                &format!(
                    "Failed to list available models: HTTP {} - {}",
                    status, body
                ),
                ctx.clone()
            ));
        }

        let models = body
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| {
                app_error!(
                    Internal,
                    "inference_failed",
                    "Failed to parse models from response",
                    ctx.clone()
                )
            })?;

        Ok(models
            .iter()
            .filter_map(|model| {
                model
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(|s| s.to_string())
            })
            .collect::<Vec<String>>())
    }
}
