// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct McpCredentials {
    pub basic: Option<McpBasicCredentials>,
    pub bearer: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct McpBasicCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct McpServer {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub url: String,
    pub credentials: Option<McpCredentials>,
}
