// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};

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
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let values = run.template_values(ctx);
            let runs = step
                .runs(ctx, self.template_engine.as_ref(), &values)
                .await
                .map_err(|err| run.failed(step, err))?;
            if !runs {
                ctx.logger
                    .info(ctx, &run.step_message(step, "skipped"))
                    .await;
                return Ok(());
            }

            ctx.logger
                .info(ctx, &run.step_message(step, "started"))
                .await;

            match &step.action {
                Action::Loop(_) => self.r#loop(ctx, step, inference, tools, run).await?,
                Action::Set(_) => run
                    .set(ctx, self.template_engine.as_ref(), step)
                    .await
                    .map_err(|err| run.failed(step, err))?,
                Action::Flow(_) => self
                    .call(ctx, step, inference, tools, run)
                    .await
                    .map_err(|err| run.failed(step, err))?,
                Action::Agent(_) => self
                    .agent(ctx, step, inference, tools, run)
                    .await
                    .map_err(|err| run.failed(step, err))?,
                Action::Tool(_) => self
                    .tool(ctx, step, tools, run)
                    .await
                    .map_err(|err| run.failed(step, err))?,
            }

            ctx.logger
                .info(ctx, &run.step_message(step, "finished"))
                .await;
            Ok(())
        })
    }
}
