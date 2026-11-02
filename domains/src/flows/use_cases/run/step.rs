// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};
use tracing::debug;

use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, FlowRun, Step},
    },
    inference::Inference,
    tools::Tools,
};

impl Flows {
    pub(super) fn step<'a>(
        &'a self,
        ctx: &'a AppContext,
        step: &'a Step,
        position: &'a str,
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let label = step.label(position);
            let values = run.template_values(ctx);
            let runs = step
                .runs(ctx, self.template_engine.as_ref(), &values)
                .await
                .map_err(|err| run.failed(label, err))?;
            if !runs {
                debug!(flow = %run.flow.metadata.name, step = %label, "Step skipped");
                return Ok(());
            }

            debug!(flow = %run.flow.metadata.name, step = %label, "Step started");

            match &step.action {
                Action::Loop(_) => {
                    self.r#loop(ctx, step, position, inference, tools, run)
                        .await?;
                }
                Action::Set(_) => run
                    .set(ctx, self.template_engine.as_ref(), step, label)
                    .await
                    .map_err(|err| run.failed(label, err))?,
                Action::Flow(_) => self
                    .call(ctx, step, inference, tools, run)
                    .await
                    .map_err(|err| run.failed(label, err))?,
                Action::Agent(_) => self
                    .agent(ctx, step, inference, tools, run)
                    .await
                    .map_err(|err| run.failed(label, err))?,
                Action::Tool(_) => self
                    .tool(ctx, step, tools, run)
                    .await
                    .map_err(|err| run.failed(label, err))?,
            }

            debug!(flow = %run.flow.metadata.name, step = %label, "Step finished");
            Ok(())
        })
    }
}
