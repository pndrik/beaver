// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::TriggerWebhook;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Triggers {
    #[serde(default)]
    pub webhook: Option<TriggerWebhook>,
}

impl Triggers {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        match &self.webhook {
            Some(webhook) => webhook.validate(ctx),
            None => Ok(()),
        }
    }
}
