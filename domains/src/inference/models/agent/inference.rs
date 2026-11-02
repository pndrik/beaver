// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::CacheLevel;
use crate::inference::models::{Compression, Model, Options};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AgentInference {
    pub model: Model,
    #[serde(default)]
    pub caching: CacheLevel,
    #[serde(default)]
    pub options: Options,
    #[serde(default)]
    pub compression: Compression,
}
