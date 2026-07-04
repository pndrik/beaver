// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Webhook {
    pub metadata: WebhookMetadata,
    pub handler: WebhookHandler,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebhookMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebhookHandler {
    pub agent: String,
    pub prompt: String,
}
