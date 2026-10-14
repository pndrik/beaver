// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Chroot {
    pub path: String,
    #[serde(default)]
    pub create: bool,
    #[serde(default)]
    pub delete: bool,
}

impl Chroot {
    pub fn validate(&self) -> Result<(), String> {
        if self.path.is_empty() {
            return Err("chroot.path is empty".to_string());
        }
        Ok(())
    }
}
