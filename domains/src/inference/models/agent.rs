// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Model;
use crate::{
    inference::models::{MessageCacheLevel, Options},
    tools::models::ToolsConfiguration,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    pub metadata: AgentMetadata,

    pub inference: AgentInference,

    #[serde(default)]
    pub tools: ToolsConfiguration,

    #[serde(default)]
    pub prompt: String,
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
            metadata: AgentMetadata {
                name,
                display_name,
                description,
            },
            inference,
            tools: ToolsConfiguration::default(),
            prompt,
        }
    }

    pub fn model(&self) -> &Model {
        &self.inference.model
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentInference {
    pub model: Model,
    #[serde(default)]
    pub caching: AgentCacheLevel,
    #[serde(default)]
    pub options: Options,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AgentCacheLevel {
    pub system: Option<MessageCacheLevel>,
    pub default: MessageCacheLevel,
}
