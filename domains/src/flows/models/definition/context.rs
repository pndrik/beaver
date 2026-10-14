// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Chroot;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub agent: String,
    #[serde(default)]
    pub chroot: Option<Chroot>,
}

impl Context {
    pub fn validate(&self) -> Result<(), String> {
        if self.agent.is_empty() {
            return Err("agent is empty".to_string());
        }
        if let Some(chroot) = &self.chroot {
            chroot.validate()?;
        }
        Ok(())
    }
}
