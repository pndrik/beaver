// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod action;
pub use action::*;

mod arguments_agent;
pub use arguments_agent::*;

mod arguments_loop;
pub use arguments_loop::*;

mod arguments_tool;
pub use arguments_tool::*;

mod chroot;
pub use chroot::*;

mod condition;
pub use condition::*;

mod context;
pub use context::*;

mod metadata;
pub use metadata::*;

mod spec;
pub use spec::*;

mod step;
pub use step::*;

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

/// A `flow_invalid` error.
pub(super) fn invalid(ctx: &AppContext, message: &str) -> AppError {
    app_error!(Validation, "flow_invalid", message, ctx.clone())
}

/// Prefixes the message of `err` with where it happened, e.g. `step 'a'`.
pub(super) fn within(location: &str, mut err: AppError) -> AppError {
    err.internal_message = format!("{}: {}", location, err.internal_message);
    err
}

/// Step and context names show up in `Steps.<step>.context`, so both must be
/// identifiers.
pub(super) fn identifier(ctx: &AppContext, kind: &str, name: &str) -> Result<(), AppError> {
    if !NAME_REGEX.is_match(name) {
        return Err(invalid(
            ctx,
            &format!("{} name '{}' must match {}", kind, name, NAME_PATTERN),
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
        self.check(ctx)
            .map_err(|err| within(&format!("Flow '{}' is invalid", self.metadata.name), err))
    }

    fn check(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.version != 1 {
            return Err(invalid(
                ctx,
                &format!("unsupported version {}, expected 1", self.version),
            ));
        }
        self.metadata.validate(ctx)?;
        self.spec.validate(ctx)
    }
}
