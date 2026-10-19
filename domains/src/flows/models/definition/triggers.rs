// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{Argument, TriggerWebhook};
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Triggers {
    #[serde(default)]
    pub webhook: Option<TriggerWebhook>,
}

impl Triggers {
    pub fn validate(
        &self,
        ctx: &AppContext,
        arguments: &BTreeMap<String, Argument>,
    ) -> Result<(), AppError> {
        match &self.webhook {
            Some(webhook) => webhook.validate(ctx, arguments),
            None => Ok(()),
        }
    }
}
