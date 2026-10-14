// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::invalid;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TriggerWebhook {
    pub name: String,
    pub token: String,
}

impl TriggerWebhook {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.name.is_empty() {
            return Err(invalid(ctx, "triggers.webhook.name is empty"));
        }
        if self.token.is_empty() {
            return Err(invalid(ctx, "triggers.webhook.token is empty"));
        }
        Ok(())
    }
}
