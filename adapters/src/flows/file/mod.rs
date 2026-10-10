// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Flow, traits::FlowProvider},
};

mod file;
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
            .configuration
            .get_string(ctx, CONFIGURATION_FLOWS_DIR)
            .await?;
        let path = path.trim_end_matches('/');

        Ok((!path.is_empty()).then(|| path.to_string()))
    }

    fn list_flow_names(&self, ctx: &AppContext, flows_dir: &str) -> Result<Vec<String>, AppError> {
        file::list_yaml_files(ctx, flows_dir)?
            .iter()
            .map(|path| {
                path.split('/')
                    .next_back()
                    .and_then(|name| name.strip_suffix(".yaml"))
                    .map(|name| name.to_string())
                    .ok_or_else(|| {
                        app_error!(
                            Internal,
                            "configuration_load_failed",
                            &format!("Invalid flow file name: {}", path),
                            ctx.clone()
                        )
                    })
            })
            .collect()
    }

    fn load_flow(
        &self,
        ctx: &AppContext,
        flows_dir: &str,
        flow_name: &str,
    ) -> Result<Flow, AppError> {
        let config_content = file::read_file(ctx, &format!("{}/{}.yaml", flows_dir, flow_name))?;
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

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Flow>, AppError> {
        let Some(flows_dir) = self.get_flows_dir(ctx).await? else {
            return Ok(vec![]);
        };

        self.list_flow_names(ctx, &flows_dir)?
            .iter()
            .map(|name| self.load_flow(ctx, &flows_dir, name))
            .collect()
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Flow, AppError> {
        let flows_dir = self.get_flows_dir(ctx).await?;
        let Some(flows_dir) =
            flows_dir.filter(|dir| file::file_exists(&format!("{}/{}.yaml", dir, name)))
        else {
            return Err(app_error!(
                NotFound,
                "flow_not_found",
                &format!("Flow '{}' not found.", name),
                ctx.clone()
            ));
        };

        self.load_flow(ctx, &flows_dir, name)
    }

    async fn get_by_webhook(&self, ctx: &AppContext, name: &str) -> Result<Option<Flow>, AppError> {
        let Some(flows_dir) = self.get_flows_dir(ctx).await? else {
            return Ok(None);
        };

        for flow_name in self.list_flow_names(ctx, &flows_dir)? {
            let flow = match self.load_flow(ctx, &flows_dir, &flow_name) {
                Ok(flow) => flow,
                Err(err) => {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!("Skipping flow '{}': {}", flow_name, err.internal_message),
                        )
                        .await;
                    continue;
                }
            };

            if flow
                .spec
                .triggers
                .webhook
                .as_ref()
                .is_some_and(|webhook| webhook.name == name)
            {
                return Ok(Some(flow));
            }
        }

        Ok(None)
    }
}
