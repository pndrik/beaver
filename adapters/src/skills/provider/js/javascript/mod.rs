// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{JsResult, JsValue, builtins::promise::PromiseState};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

mod base64;
mod fetch;
use fetch::FilteredFetcher;
mod helper;
mod value;
pub use value::Value;

use crate::skills::provider::js::models::Scopes;

pub struct JavaScript {
    directory: String,
    entrypoint: String,
    scopes: Scopes,
}

impl JavaScript {
    pub fn new(directory: String, entrypoint: String, scopes: Scopes) -> Self {
        Self {
            directory,
            entrypoint,
            scopes,
        }
    }

    pub fn call(
        &mut self,
        ctx: &AppContext,
        function: &str,
        input: Vec<Value>,
    ) -> Result<Value, AppError> {
        let mut context = self.get_context(ctx)?;
        let mut module = self.get_module(ctx, &mut context)?;
        let function = self.get_function(ctx, &mut context, &mut module, function)?;
        let arguments = input
            .into_iter()
            .map(|v| v.into_js(&mut context))
            .collect::<JsResult<Vec<_>>>()
            .map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed converting arguments to JavaScript: {}", e),
                    ctx.clone()
                )
            })?;

        let result = function
            .call(
                &JsValue::undefined(), // this
                &arguments,
                &mut context,
            )
            .map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed executing JavaScript: {}", e),
                    ctx.clone()
                )
            })?;
        context.run_jobs().map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to run jobs: {}", e),
                ctx.clone()
            )
        })?;

        let resolved = match result.as_promise() {
            Some(p) => match p.state() {
                PromiseState::Fulfilled(v) => v,
                PromiseState::Rejected(e) => {
                    return Err(app_error!(
                        Internal,
                        "skill_failed",
                        &format!("JavaScript promise rejected: {}", e.display()),
                        ctx.clone()
                    ));
                }
                PromiseState::Pending => {
                    return Err(app_error!(
                        Internal,
                        "skill_failed",
                        "JavaScript promise is still pending",
                        ctx.clone()
                    ));
                }
            },
            None => result,
        };

        Value::new_from_js_value(&resolved, &mut context).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed converting result to Value: {}", e),
                ctx.clone()
            )
        })
    }
}
