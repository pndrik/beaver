// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::collections::HashMap;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, ToolPermission},
        traits::ToolsProvider,
    },
};

mod javascript;
use javascript::{JavaScript, Value};
mod file;
mod helper;
mod models;

use models::Tool;

const CONFIGURATION_SKILLS_DIR: &str = "tools.providers.javascript.path";
const CONFIGURATION_SKILL_CONFIGURATION: &str = "tools.configuration";

pub struct JavascriptProvider {
    tools: HashMap<String, Tool>,
}

impl JavascriptProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
}

#[async_trait]
impl ToolsProvider for JavascriptProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let dirs = file::list_directories(ctx, &self.get_tools_path(ctx).await?)?;
        self.tools.clear();

        for dir in dirs {
            let package = match self.load_package_info(ctx, &dir).await {
                Ok(pkg) => pkg,
                Err(_) => {
                    continue;
                }
            };

            for (name, tool) in package.config.beaver.tools {
                self.tools.insert(
                    name.clone(),
                    Tool {
                        directory: dir.clone(),
                        package: package.name.clone(),
                        main: tool.main.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                        scopes: tool.scopes.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    async fn list(
        &self,
        _ctx: &AppContext,
    ) -> Result<Vec<app_domains::tools::models::Tool>, AppError> {
        self.tools
            .iter()
            .map(|(name, tool)| {
                Ok(app_domains::tools::models::Tool {
                    name: format!("js_{}", name),
                    description: tool.description.clone(),
                    schema: tool.parameters.clone(),
                })
            })
            .collect::<Result<Vec<app_domains::tools::models::Tool>, AppError>>()
    }

    async fn get(
        &self,
        ctx: &AppContext,
        name: &str,
    ) -> Result<app_domains::tools::models::Tool, AppError> {
        let name = name.strip_prefix("js_").unwrap_or(name);
        let Some(tool) = self.tools.get(name) else {
            return Err(app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool with name '{}' not found", name),
                ctx.clone()
            ));
        };

        Ok(app_domains::tools::models::Tool {
            name: format!("js_{}", name),
            description: tool.description.clone(),
            schema: tool.parameters.clone(),
        })
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let name = input.name.strip_prefix("js_").unwrap_or(&input.name);
        let Some(tool) = self.tools.get(name) else {
            return Err(app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool with name '{}' not found", name),
                ctx.clone()
            ));
        };

        // TBD: This has room for improvement
        let configuration = self.get_tool_configuration(ctx, &tool.package).await?;
        let input_value = self.input_to_value(&input);
        let permissions_value = self.permission_to_value(&permissions);
        let ctx_clone = ctx.clone();
        let directory = tool.directory.clone();
        let main = tool.main.clone();
        let scopes = tool.scopes.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut js =
                JavaScript::new(directory.clone(), format!("{}/{}", directory, main), scopes);

            js.call(
                &ctx_clone,
                "main",
                vec![input_value, configuration, permissions_value],
            )
        })
        .await
        .map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Tool '{}' execution failed: {}", input.name, e),
                ctx.clone()
            )
        })??;

        Ok(match result {
            Value::String(s) => s,
            _ => {
                return Err(app_error!(
                    Internal,
                    "tool_failed",
                    &format!(
                        "Tool '{}' execution returned non-string result.",
                        input.name
                    ),
                    ctx.clone()
                ));
            }
        })
    }
}
