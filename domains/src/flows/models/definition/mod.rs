// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod action;
pub use action::*;

mod argument;
pub use argument::*;

mod arguments_agent;
pub use arguments_agent::*;

mod arguments_flow;
pub use arguments_flow::*;

mod arguments_loop;
pub use arguments_loop::*;

mod arguments_set;
pub use arguments_set::*;

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

mod value_type;
pub use value_type::*;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::LazyLock;

use crate::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::TemplateEngine,
    },
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

    /// Checks `raw` against `spec.arguments` and returns the typed arguments.
    /// Fails with `flow_arguments_invalid` for undeclared, missing required,
    /// unconvertible or non-matching arguments. Typed values pass unchanged.
    pub fn arguments(
        &self,
        ctx: &AppContext,
        raw: &Map<String, Value>,
    ) -> Result<Map<String, Value>, AppError> {
        let declared = &self.spec.arguments;
        let mut problems: Vec<String> = raw
            .keys()
            .filter(|name| !declared.contains_key(*name))
            .map(|name| format!("argument '{}' is not declared", name))
            .collect();

        let mut typed = Map::new();
        for (name, argument) in declared {
            match raw.get(name) {
                Some(value) => match argument.parse(ctx, value) {
                    Ok(value) => {
                        typed.insert(name.clone(), value);
                    }
                    Err(err) if err.translation_key == "flow_arguments_invalid" => {
                        problems.push(format!("argument '{}': {}", name, err.internal_message))
                    }
                    Err(err) => return Err(err),
                },
                None if argument.required => {
                    problems.push(format!("argument '{}' is required", name))
                }
                None => {}
            }
        }

        if !problems.is_empty() {
            return Err(app_error!(
                Validation,
                "flow_arguments_invalid",
                &format!(
                    "Flow '{}' got invalid arguments: {}",
                    self.metadata.name,
                    problems.join("; ")
                ),
                ctx.clone()
            ));
        }
        Ok(typed)
    }

    /// The typed arguments from a webhook payload, see
    /// [`TriggerWebhook::render_arguments`] and [`Definition::arguments`].
    pub async fn webhook_arguments(
        &self,
        ctx: &AppContext,
        engine: &(dyn TemplateEngine + Send + Sync),
        body: &Value,
        query: &Value,
    ) -> Result<Map<String, Value>, AppError> {
        let raw = match &self.spec.triggers.webhook {
            Some(webhook) => {
                webhook
                    .render_arguments(ctx, engine, &self.metadata.name, body, query)
                    .await?
            }
            None => Map::new(),
        };
        self.arguments(ctx, &raw)
    }
}
