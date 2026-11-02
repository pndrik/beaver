// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{AgentInference, invalid};
use crate::{
    core::models::{AppContext, AppError},
    tools::models::ToolsConfiguration,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub inference: AgentInference,
    #[serde(default)]
    pub tools: ToolsConfiguration,
    pub prompt: String,
}

impl Spec {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.prompt.trim().is_empty() {
            return Err(invalid(ctx, "spec.prompt is empty"));
        }
        self.tools.validate(ctx, "spec.tools")
    }
}
