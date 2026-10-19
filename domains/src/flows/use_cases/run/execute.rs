// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin, sync::Arc};

use super::chroot;
use crate::{
    core::models::{AppContext, AppError},
    flows::{Flows, models::FlowRun},
    inference::Inference,
    tools::Tools,
};

impl Flows {
    pub(super) fn execute<'a>(
        &'a self,
        ctx: &'a AppContext,
        run: &'a mut FlowRun,
        inference: &'a Inference,
        tools: &'a Tools,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let flow = Arc::clone(&run.flow);
            let flow_name = &flow.metadata.name;
            ctx.logger
                .info(ctx, &format!("Flow '{}' started", flow_name))
                .await;

            let result = async {
                self.contexts(ctx, inference, tools, run).await?;
                for step in &flow.spec.steps {
                    self.step(ctx, step, inference, tools, run).await?;
                }
                Ok::<(), AppError>(())
            }
            .await;

            for chroot in &run.delete_on_finish {
                chroot::cleanup(ctx, chroot).await;
            }

            if result.is_ok() {
                ctx.logger
                    .info(ctx, &format!("Flow '{}' finished", flow_name))
                    .await;
            }

            result
        })
    }
}
