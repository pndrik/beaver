// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod action;
pub use action::*;

mod agent_arguments;
pub use agent_arguments::*;

mod chroot;
pub use chroot::*;

mod context;
pub use context::*;

mod metadata;
pub use metadata::*;

mod spec;
pub use spec::*;

mod step;
pub use step::*;

mod tool_arguments;
pub use tool_arguments::*;

mod trigger_webhook;
pub use trigger_webhook::*;

mod triggers;
pub use triggers::*;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

const NAME_PATTERN: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

static NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(NAME_PATTERN).expect("valid name pattern"));

/// Step and context names show up in `Steps.<step>.context`, so both must be
/// identifiers.
pub(super) fn identifier(kind: &str, name: &str) -> Result<(), String> {
    if !NAME_REGEX.is_match(name) {
        return Err(format!(
            "{} name '{}' must match {}",
            kind, name, NAME_PATTERN
        ));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub version: u32,
    pub metadata: Metadata,
    pub spec: Spec,
}

impl Definition {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        self.check().map_err(|message| {
            app_error!(
                Validation,
                "flow_invalid",
                &format!("Flow '{}' is invalid: {}", self.metadata.name, message),
                ctx.clone()
            )
        })
    }

    fn check(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("unsupported version {}, expected 1", self.version));
        }
        self.metadata.validate()?;
        self.spec.validate()
    }
}
