// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

use super::super::permissions;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError, LiteralOrRegex},
    inference::models::Conversation,
    tools::{
        models::{
            Call, SandboxFetchScope, SandboxFilesystemOperation, SandboxFilesystemScope,
            SandboxLimits, SandboxOutput, SandboxRun, SandboxScopes, SandboxSource, Schema,
            SchemaField, SchemaFieldType, ToolPermission,
        },
        traits::{CodeSandbox, Tool},
    },
};

const NAME: &str = "javascript_run";
const DESCRIPTION: &str = r#"Runs JavaScript on chroot files without loading them into the conversation. Pass `code` or `path`.

- `code`: async function body, top-level `await`/`return`, `await import("/lib/x.js")`.
- `path`: `.js` module, static `import`. Result is the default export, functions are awaited. Use `export default async function () {}`, not an async arrow.

Returns `console.*` lines, then the value (string as is, JSON otherwise). Errors include the console output.

- `fs.readFile(path)`, `fs.readDir(path)` (directories end with `/`), `fs.writeFile(path, content)`, `fs.appendFile(path, content)`
- `fetch` only for URLs the permission allows

Paths are absolute, `/` is the chroot. Only chroot files can be imported, no packages. Loops and recursion are limited."#;

const CONFIGURATION_LOOP_ITERATION_LIMIT: &str =
    "tools.configuration.javascript.loop_iteration_limit";
const CONFIGURATION_RECURSION_LIMIT: &str = "tools.configuration.javascript.recursion_limit";
const DEFAULT_LOOP_ITERATION_LIMIT: u64 = 100_000_000;
const DEFAULT_RECURSION_LIMIT: usize = 512;

#[derive(Debug, Deserialize)]
struct Arguments {
    pub code: Option<String>,
    pub path: Option<String>,
}

pub struct Run {
    sandbox: Arc<dyn CodeSandbox + Send + Sync>,
}

impl Run {
    pub fn new(sandbox: Arc<dyn CodeSandbox + Send + Sync>) -> Self {
        Self { sandbox }
    }
}

async fn limit(ctx: &AppContext, key: &str) -> Option<u64> {
    let value = ctx.configuration.get_int(ctx, key).await.ok()?;
    u64::try_from(value).ok().filter(|value| *value > 0)
}

async fn limits(ctx: &AppContext) -> SandboxLimits {
    SandboxLimits {
        loop_iterations: limit(ctx, CONFIGURATION_LOOP_ITERATION_LIMIT)
            .await
            .unwrap_or(DEFAULT_LOOP_ITERATION_LIMIT),
        recursion: limit(ctx, CONFIGURATION_RECURSION_LIMIT)
            .await
            .map_or(DEFAULT_RECURSION_LIMIT, |limit| limit as usize),
    }
}

fn format_output(ctx: &AppContext, output: SandboxOutput) -> Result<String, AppError> {
    let console = output.console.trim_end_matches('\n');
    let value = output.result.map_err(|e| {
        let message = match console.is_empty() {
            true => format!("Script failed: {}", e),
            false => format!("Script failed: {}\nConsole output:\n{}", e, console),
        };
        app_error!(Validation, "tool_failed", &message, ctx.clone())
    })?;

    let value = match value {
        None => String::new(),
        Some(serde_json::Value::String(value)) => value,
        Some(value) => value.to_string(),
    };
    Ok([console, value.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[async_trait]
impl Tool for Run {
    async fn name(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(NAME.to_string())
    }

    async fn description(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(DESCRIPTION.to_string())
    }

    async fn schema(&self, _ctx: &AppContext) -> Result<Schema, AppError> {
        let mut schema = Schema::new(NAME, "");

        schema.add_property(
            "code",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "JavaScript to run as the body of an async function. Use either `code` or `path`.",
                None,
            ),
        );

        schema.add_property(
            "path",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Absolute path within the current chroot to a `.js` module to run. Use either `code` or `path`.",
                None,
            ),
        );

        Ok(schema)
    }

    async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let args: Arguments = input.arguments_into().map_err(|e| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Invalid arguments for tool '{}': {}", NAME, e),
                ctx.clone()
            )
        })?;

        let source = match (args.code, args.path) {
            (Some(code), None) => SandboxSource::Code(code),
            (None, Some(path)) if path.starts_with('/') => SandboxSource::File(path),
            (None, Some(path)) => {
                return Err(app_error!(
                    Validation,
                    "tool_failed",
                    &format!("Path must be absolute: '{}'", path),
                    ctx.clone()
                ));
            }
            _ => {
                return Err(app_error!(
                    Validation,
                    "tool_failed",
                    "Pass exactly one of 'code' or 'path'",
                    ctx.clone()
                ));
            }
        };

        let fetch = permissions::web_patterns(ctx, &permissions)?
            .into_iter()
            .map(|url| SandboxFetchScope {
                url: LiteralOrRegex::Regex(url),
                methods: vec![],
            })
            .collect();
        let filesystem = vec![SandboxFilesystemScope {
            path: LiteralOrRegex::Literal("/".to_string()),
            operations: vec![
                SandboxFilesystemOperation::Read,
                SandboxFilesystemOperation::Write,
            ],
        }];
        let run = SandboxRun {
            source,
            scopes: SandboxScopes { fetch, filesystem },
            limits: Some(limits(ctx).await),
        };
        let output = self.sandbox.run(ctx, conversation.chroot(), run).await?;

        format_output(ctx, output)
    }
}
