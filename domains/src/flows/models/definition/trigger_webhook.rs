// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TriggerWebhook {
    pub name: String,
    pub token: String,
}

impl TriggerWebhook {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("triggers.webhook.name is empty".to_string());
        }
        if self.token.is_empty() {
            return Err("triggers.webhook.token is empty".to_string());
        }
        Ok(())
    }
}
