// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use super::{Argument, invalid};
use crate::core::{
    models::{AppContext, AppError},
    traits::TemplateEngine,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TriggerWebhook {
    pub name: String,
    pub token: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
}

impl TriggerWebhook {
    pub fn validate(
        &self,
        ctx: &AppContext,
        arguments: &BTreeMap<String, Argument>,
    ) -> Result<(), AppError> {
        if self.name.is_empty() {
            return Err(invalid(ctx, "triggers.webhook.name is empty"));
        }
        if self.token.is_empty() {
            return Err(invalid(ctx, "triggers.webhook.token is empty"));
        }
        if let Some(name) = self
            .arguments
            .keys()
            .find(|name| !arguments.contains_key(*name))
        {
            return Err(invalid(
                ctx,
                &format!(
                    "triggers.webhook.arguments maps unknown argument '{}'",
                    name
                ),
            ));
        }
        if let Some((name, _)) = arguments
            .iter()
            .find(|(name, argument)| argument.required && !self.arguments.contains_key(*name))
        {
            return Err(invalid(
                ctx,
                &format!(
                    "triggers.webhook.arguments does not map required argument '{}'",
                    name
                ),
            ));
        }
        Ok(())
    }

    /// Renders the mapping against the payload. These templates are the only
    /// ones that see `Body` and `Query`.
    pub async fn render_arguments(
        &self,
        ctx: &AppContext,
        engine: &(dyn TemplateEngine + Send + Sync),
        flow_name: &str,
        body: &Value,
        query: &Value,
    ) -> Result<Map<String, Value>, AppError> {
        let values = json!({
            "Body": body,
            "Query": query,
            "Ctx": ctx,
            "Flow": { "name": flow_name },
        });

        let mut raw = Map::new();
        for (name, template) in &self.arguments {
            let rendered = engine.render(ctx, template, &values).await?;
            raw.insert(name.clone(), Value::String(rendered));
        }
        Ok(raw)
    }
}
