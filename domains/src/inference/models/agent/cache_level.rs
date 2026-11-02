// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use crate::inference::models::MessageCacheLevel;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CacheLevel {
    pub system: Option<MessageCacheLevel>,
    pub default: MessageCacheLevel,
}
