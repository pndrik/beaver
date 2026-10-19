// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ValueType, invalid, within};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::helpers::regex,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Argument {
    #[serde(rename = "type", default)]
    pub type_: ValueType,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub required: bool,
}

impl Argument {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if let Some(pattern) = &self.regex {
            if self.type_ != ValueType::String {
                return Err(invalid(ctx, "regex is only allowed for type string"));
            }
            regex(ctx, pattern).map_err(|err| within("regex", err))?;
        }
        Ok(())
    }

    pub fn parse(&self, ctx: &AppContext, value: &Value) -> Result<Value, AppError> {
        let rejected = |message: String| {
            app_error!(Validation, "flow_arguments_invalid", &message, ctx.clone())
        };
        let parsed = self
            .type_
            .parse(value)
            .ok_or_else(|| rejected(format!("{} is not a {}", value, self.type_.name())))?;
        if let (Some(pattern), Value::String(text)) = (&self.regex, &parsed)
            && !regex(ctx, pattern)?.is_match(text)
        {
            return Err(rejected(format!("{} does not match '{}'", value, pattern)));
        }
        Ok(parsed)
    }
}
