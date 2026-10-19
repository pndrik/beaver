// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};

use super::Definition;

/// A resolved webhook call: the flow and its typed arguments.
#[derive(Debug, Clone)]
pub struct Hook {
    pub flow: Definition,
    pub arguments: Map<String, Value>,
}
