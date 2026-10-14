// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::PathBuf;

use serde_json::Value;

use super::{RunContext, chroot};
use crate::{
    core::models::{AppContext, AppError},
    flows::{Flows, models::Context},
    inference::Inference,
    tools::Tools,
};

impl Flows {
    pub(super) async fn context(
        &self,
        ctx: &AppContext,
        declaration: &Context,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
        delete_on_finish: &mut Vec<PathBuf>,
    ) -> Result<RunContext, AppError> {
        let chroot = match &declaration.chroot {
            Some(declared) => {
                let path = self
                    .template_engine
                    .render(ctx, &declared.path, values)
                    .await?;
                let chroot = chroot::prepare(ctx, &path, declared.create, declared.delete).await?;
                if declared.delete && !delete_on_finish.contains(&chroot) {
                    delete_on_finish.push(chroot.clone());
                }
                chroot
            }
            None => chroot::global_chroot(ctx).await?,
        };

        let conversation = inference
            .new_conversation(ctx, &declaration.agent, tools, &chroot.to_string_lossy())
            .await?;

        Ok(RunContext {
            chroot,
            conversation,
        })
    }
}
