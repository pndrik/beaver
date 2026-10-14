// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Action, Context};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Step {
    pub name: String,
    pub context: Context,
    pub action: Action,
}
