// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Step, Triggers};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Spec {
    #[serde(default)]
    pub triggers: Triggers,
    pub steps: Vec<Step>,
}
