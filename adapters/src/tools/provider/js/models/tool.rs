// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::FetchScope;
use app_domains::tools::models::Schema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub directory: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub package: String,

    pub main: String,
    pub description: String,
    pub scopes: Scopes,
    pub parameters: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scopes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch: Option<FetchScope>,
}
