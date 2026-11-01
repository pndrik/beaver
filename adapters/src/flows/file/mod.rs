// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use tracing::warn;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{models::Definition, traits::FlowProvider},
};

use super::token;

mod io;
mod parse;

#[cfg(test)]
mod tests;

const CONFIGURATION_FLOWS_DIR: &str = "flows.path";

pub struct File {}

impl File {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {}
    }

    async fn get_flows_dir(&self, ctx: &AppContext) -> Result<Option<String>, AppError> {
        let path = ctx
            .configuration()
            .get_string(ctx, CONFIGURATION_FLOWS_DIR)
            .await?;
        let path = path.trim_end_matches('/');

        Ok((!path.is_empty()).then(|| path.to_string()))
    }

    fn list_flow_names(ctx: &AppContext, flows_dir: &str) -> Result<Vec<String>, AppError> {
        io::list_yaml_files(ctx, flows_dir)?
            .iter()
            .map(|path| {
                path.split('/')
                    .next_back()
                    .and_then(|name| name.strip_suffix(".yaml"))
                    .map(std::string::ToString::to_string)
                    .ok_or_else(|| {
                        app_error!(
                            Internal,
                            "configuration_load_failed",
                            &format!("Invalid flow file name: {path}"),
                            ctx.clone()
                        )
                    })
            })
            .collect()
    }

    fn load_flow(
        ctx: &AppContext,
        flows_dir: &str,
        flow_name: &str,
    ) -> Result<Definition, AppError> {
        let config_content = io::read_file(ctx, &format!("{flows_dir}/{flow_name}.yaml"))?;
        let flow = parse::parse_flow(ctx, &config_content)?;

        if !flow.metadata.name.is_empty() && flow.metadata.name != flow_name {
            return Err(app_error!(
                Validation,
                "flow_invalid",
                &format!(
                    "Flow name mismatch: expected '{}', found '{}'.",
                    flow_name, flow.metadata.name
                ),
                ctx.clone()
            ));
        }

        flow.validate(ctx)?;

        Ok(flow)
    }
}

#[async_trait]
impl FlowProvider for File {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Definition>, AppError> {
        let Some(flows_dir) = self.get_flows_dir(ctx).await? else {
            return Ok(vec![]);
        };

        File::list_flow_names(ctx, &flows_dir)?
            .iter()
            .map(|name| File::load_flow(ctx, &flows_dir, name))
            .collect()
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Definition, AppError> {
        let flows_dir = self.get_flows_dir(ctx).await?;
        let Some(flows_dir) =
            flows_dir.filter(|dir| io::file_exists(&format!("{dir}/{name}.yaml")))
        else {
            return Err(app_error!(
                NotFound,
                "flow_not_found",
                &format!("Flow '{name}' not found."),
                ctx.clone()
            ));
        };

        File::load_flow(ctx, &flows_dir, name)
    }

    async fn get_by_webhook(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
    ) -> Result<Option<Definition>, AppError> {
        let Some(flows_dir) = self.get_flows_dir(ctx).await? else {
            return Ok(None);
        };

        for flow_name in File::list_flow_names(ctx, &flows_dir)? {
            let flow = match File::load_flow(ctx, &flows_dir, &flow_name) {
                Ok(flow) => flow,
                Err(err) => {
                    warn!(flow = %flow_name, error = %err.internal_message, "Skipping flow");
                    continue;
                }
            };

            if token::has_webhook(&flow, name) {
                return Ok(token::token_matches(&flow, token).then_some(flow));
            }
        }

        Ok(None)
    }
}
