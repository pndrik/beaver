// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use minijinja::{
    AutoEscape, Environment, Error, ErrorKind, UndefinedBehavior, escape_formatter,
    syntax::SyntaxConfig,
    value::{Serde, Value as TemplateValue, ValueKind},
};
use serde_json::Value;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    core::traits::TemplateEngine,
};

#[cfg(test)]
mod tests;

const TEMPLATE_NAME: &str = "template";
const UNDEFINED_PLACEHOLDER: &str = "<NULL>";
const LAST_METHOD: &str = "last";

pub struct MiniJinjaTemplateEngine {}

impl MiniJinjaTemplateEngine {
    pub fn new(_ctx: &AppContext) -> Self {
        MiniJinjaTemplateEngine {}
    }
}

fn new_environment(ctx: &AppContext) -> Result<Environment<'static>, AppError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Chainable);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.set_syntax(
        SyntaxConfig::builder()
            .keep_trailing_newline(true)
            .build()
            .map_err(|e| {
                app_error!(
                    Internal,
                    "template_failed",
                    &format!("Failed to build template syntax: {}", e),
                    ctx.clone()
                )
            })?,
    );
    env.set_formatter(|out, state, value| {
        if value.is_undefined() || value.is_none() {
            out.write_str(UNDEFINED_PLACEHOLDER)?;
            return Ok(());
        }
        escape_formatter(out, state, value)
    });
    env.set_unknown_method_callback(|_, value, method, args| {
        if method == LAST_METHOD && args.is_empty() {
            return last(value);
        }
        Err(Error::from(ErrorKind::UnknownMethod))
    });

    Ok(env)
}

fn last(value: &TemplateValue) -> Result<TemplateValue, Error> {
    match value.kind() {
        ValueKind::Undefined => Ok(TemplateValue::UNDEFINED),
        ValueKind::Seq => match value.len() {
            Some(len) if len > 0 => value.get_item_by_index(len - 1),
            _ => Ok(TemplateValue::UNDEFINED),
        },
        _ => Err(Error::from(ErrorKind::UnknownMethod)),
    }
}

fn path_exists(values: &Value, path: &str) -> bool {
    let mut current = values;
    for segment in path.split('.') {
        // `undeclared_variables` reports `a.last()` as `a.last`.
        if current.is_array() && segment == LAST_METHOD {
            return true;
        }
        match current.get(segment) {
            Some(next) => current = next,
            None => return false,
        }
    }
    true
}

fn missing_paths(
    env: &Environment,
    referenced: impl IntoIterator<Item = String>,
    values: &Value,
) -> Vec<String> {
    let mut paths: Vec<String> = referenced
        .into_iter()
        .filter(|path| {
            let root = path.split('.').next().unwrap_or_default();
            !env.globals().any(|(name, _)| name == root)
        })
        .collect();
    paths.sort();

    let mut missing: Vec<String> = Vec::new();
    for path in paths {
        if missing
            .iter()
            .any(|m| path.starts_with(m.as_str()) && path[m.len()..].starts_with('.'))
        {
            continue;
        }
        if path_exists(values, &path) {
            continue;
        }
        let path = match path.strip_suffix(&format!(".{}", LAST_METHOD)) {
            Some(list) if !path_exists(values, list) => list.to_string(),
            _ => path,
        };
        if !missing.contains(&path) {
            missing.push(path);
        }
    }

    missing
}

#[async_trait]
impl TemplateEngine for MiniJinjaTemplateEngine {
    async fn render(
        &self,
        ctx: &AppContext,
        template: &str,
        values: &Value,
    ) -> Result<String, AppError> {
        let mut env = new_environment(ctx)?;
        env.add_template_owned(TEMPLATE_NAME, template.to_string())
            .map_err(|e| {
                app_error!(
                    Internal,
                    "template_failed",
                    &format!("Failed to parse template: {}", e),
                    ctx.clone()
                )
            })?;
        let tmpl = env.get_template(TEMPLATE_NAME).map_err(|e| {
            app_error!(
                Internal,
                "template_failed",
                &format!("Failed to load template: {}", e),
                ctx.clone()
            )
        })?;

        for path in missing_paths(&env, tmpl.undeclared_variables(true), values) {
            ctx.logger
                .warn(
                    ctx,
                    &format!(
                        "Template references undefined value '{}', rendering {}",
                        path, UNDEFINED_PLACEHOLDER
                    ),
                )
                .await;
        }

        tmpl.render(minijinja::Value::from(Serde(values)))
            .map_err(|e| {
                app_error!(
                    Internal,
                    "template_failed",
                    &format!("Failed to render template: {}", e),
                    ctx.clone()
                )
            })
    }
}
