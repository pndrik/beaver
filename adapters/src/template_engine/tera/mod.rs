// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde_json::Value;
use tera::{Context, Tera};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    core::traits::TemplateEngine,
};

pub struct TeraTemplateEngine {}

impl TeraTemplateEngine {
    pub fn new(_ctx: &AppContext) -> Self {
        TeraTemplateEngine {}
    }
}

#[async_trait]
impl TemplateEngine for TeraTemplateEngine {
    async fn render(
        &self,
        ctx: &AppContext,
        template: &str,
        values: &Value,
    ) -> Result<String, AppError> {
        let context = Context::from_serialize(values).map_err(|e| {
            app_error!(
                Internal,
                "template_failed",
                &format!("Failed to build Tera context: {}", e),
                ctx.clone()
            )
        })?;

        Tera::one_off(template, &context, true).map_err(|e| {
            app_error!(
                Internal,
                "template_failed",
                &format!("Failed to render template: {}", e),
                ctx.clone()
            )
        })
    }
}
