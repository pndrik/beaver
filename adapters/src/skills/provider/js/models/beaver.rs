// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Skill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beaver {
    pub skills: HashMap<String, Skill>,
}
