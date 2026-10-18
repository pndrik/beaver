// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Model;

/// Summarizes the conversation once the maximum of `tokens` or `messages` is
/// reached, whichever comes first.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Compression {
    #[serde(default)]
    pub enabled: bool,
    /// Defaults to the agent model.
    #[serde(default)]
    pub model: Option<Model>,
    /// Defaults to a built-in prompt.
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub tokens: Option<u64>,
    #[serde(default)]
    pub messages: Option<u64>,
}
