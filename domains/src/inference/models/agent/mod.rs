// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod cache_level;
pub use cache_level::*;

mod inference;
pub use inference::*;

mod metadata;
pub use metadata::*;

mod spec;
pub use spec::*;

use serde::{Deserialize, Serialize};

use super::Model;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::ToolsConfiguration,
};

pub(crate) fn invalid(ctx: &AppContext, message: &str) -> AppError {
    app_error!(Validation, "agent_invalid", message, ctx.clone())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Agent {
    pub version: u32,
    pub metadata: Metadata,
    pub spec: Spec,
}

impl Agent {
    pub fn new(
        name: String,
        display_name: String,
        description: String,
        prompt: String,
        inference: AgentInference,
    ) -> Self {
        Self {
            version: 1,
            metadata: Metadata {
                name,
                display_name,
                description,
            },
            spec: Spec {
                inference,
                tools: ToolsConfiguration::default(),
                prompt,
            },
        }
    }

    pub fn model(&self) -> &Model {
        &self.spec.inference.model
    }

    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        self.check(ctx).map_err(|err| {
            invalid(
                ctx,
                &format!(
                    "Agent '{}' is invalid: {}",
                    self.metadata.name, err.internal_message
                ),
            )
        })
    }

    fn check(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.version != 1 {
            return Err(invalid(
                ctx,
                &format!("unsupported version {}, expected 1", self.version),
            ));
        }
        self.metadata.validate(ctx)?;
        self.spec.validate(ctx)
    }
}
