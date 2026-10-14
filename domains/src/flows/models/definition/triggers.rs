// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::TriggerWebhook;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Triggers {
    #[serde(default)]
    pub webhook: Option<TriggerWebhook>,
}
