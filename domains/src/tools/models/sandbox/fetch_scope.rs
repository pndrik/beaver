// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use crate::core::models::LiteralOrRegex;

/// Allows web requests to `url` with one of `methods`, any method if empty.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SandboxFetchScope {
    pub url: LiteralOrRegex,
    #[serde(default)]
    pub methods: Vec<String>,
}

impl SandboxFetchScope {
    pub fn allows_url(&self, url: &str) -> bool {
        self.url.matches(url)
    }

    pub fn allows(&self, url: &str, method: &str) -> bool {
        self.allows_url(url)
            && (self.methods.is_empty()
                || self
                    .methods
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(method)))
    }
}
