// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::{ValueType, identifier, invalid, within};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::helpers::regex,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ArgumentsSet(pub BTreeMap<String, SetVariable>);

impl ArgumentsSet {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.0.is_empty() {
            return Err(invalid(ctx, "arguments is empty"));
        }
        for (name, variable) in &self.0 {
            identifier(ctx, "variable", name)?;
            variable
                .validate(ctx)
                .map_err(|err| within(&format!("variable '{}'", name), err))?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetVariable {
    pub value: String,
    #[serde(rename = "type", default)]
    pub type_: ValueType,
    #[serde(default)]
    pub extract: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,
}

impl SetVariable {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if let Some(extract) = &self.extract {
            regex(ctx, extract).map_err(|err| within("extract", err))?;
        }
        if let Some(default) = &self.default
            && self.type_.parse(default).is_none()
        {
            return Err(invalid(
                ctx,
                &format!("default {} is not a {}", default, self.type_.name()),
            ));
        }
        Ok(())
    }

    pub fn resolve(&self, ctx: &AppContext, rendered: &str) -> Result<Value, AppError> {
        let rejected = |message: String| {
            app_error!(Validation, "flow_variable_invalid", &message, ctx.clone())
        };
        let text = match &self.extract {
            Some(extract) => {
                let regex = regex(ctx, extract)?;
                let captures = regex.captures(rendered).ok_or_else(|| {
                    rejected(format!("'{}' does not match '{}'", rendered, extract))
                })?;
                let group = if regex.captures_len() > 1 { 1 } else { 0 };
                captures
                    .get(group)
                    .ok_or_else(|| rejected(format!("group 1 of '{}' did not match", extract)))?
                    .as_str()
            }
            None => rendered,
        };
        self.type_
            .parse(&Value::from(text))
            .ok_or_else(|| rejected(format!("'{}' is not a {}", text, self.type_.name())))
    }

    pub fn fallback(&self) -> Value {
        self.default
            .as_ref()
            .and_then(|default| self.type_.parse(default))
            .unwrap_or_else(|| self.type_.zero())
    }
}
