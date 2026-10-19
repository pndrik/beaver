// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use super::chroot;
use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{FlowRun, RunContext},
    },
    inference::Inference,
    tools::Tools,
};

impl Flows {
    pub(super) async fn contexts(
        &self,
        ctx: &AppContext,
        inference: &Inference,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        let flow = Arc::clone(&run.flow);
        let values = run.template_values(ctx);

        for (name, declaration) in &flow.spec.contexts {
            let failed = |mut err: AppError| {
                err.internal_message = format!(
                    "Flow '{}' context '{}' failed: {}",
                    flow.metadata.name, name, err.internal_message
                );
                err
            };

            let chroot = match &declaration.chroot {
                Some(declared) => {
                    let path = self
                        .template_engine
                        .render(ctx, &declared.path, &values)
                        .await
                        .map_err(failed)?;
                    let chroot = chroot::prepare(ctx, &path, declared.create, declared.delete)
                        .await
                        .map_err(failed)?;
                    if declared.delete {
                        run.delete_when_finished(chroot.clone());
                    }
                    chroot
                }
                None => chroot::global_chroot(ctx).await.map_err(failed)?,
            };

            let conversation = inference
                .new_conversation(ctx, &declaration.agent, tools, &chroot.to_string_lossy())
                .await
                .map_err(failed)?;
            run.contexts
                .insert(name.clone(), RunContext { conversation });
        }
        Ok(())
    }
}
