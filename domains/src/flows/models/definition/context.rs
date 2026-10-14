// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Chroot;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Context {
    pub name: String,
    #[serde(default)]
    pub chroot: Option<Chroot>,
}
