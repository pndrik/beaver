// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{identifier, invalid};
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsFlow {
    pub flow: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
}

impl ArgumentsFlow {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.flow.is_empty() {
            return Err(invalid(ctx, "arguments.flow is empty"));
        }
        self.arguments
            .keys()
            .try_for_each(|name| identifier(ctx, "argument", name))
    }
}
