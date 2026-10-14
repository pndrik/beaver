// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use super::{FlowRun, conditions, values};
use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, ArgumentsLoop, Definition, Step},
    },
    inference::Inference,
    tools::Tools,
};

fn failed(flow: &Definition, step: &Step, mut err: AppError) -> AppError {
    err.internal_message = format!(
        "Flow '{}' step '{}' failed: {}",
        flow.metadata.name, step.name, err.internal_message
    );
    err
}

impl Flows {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn steps<'a>(
        &'a self,
        ctx: &'a AppContext,
        flow: &'a Definition,
        body: &'a Value,
        query: &'a Value,
        steps: &'a [Step],
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let flow_name = &flow.metadata.name;
            for step in steps {
                let values = values::template_values(ctx, flow, body, query, &run.steps);
                if let Some(when) = &step.when
                    && !conditions::any(self.template_engine.as_ref(), ctx, when, &values)
                        .await
                        .map_err(|err| failed(flow, step, err))?
                {
                    ctx.logger
                        .info(
                            ctx,
                            &format!("Flow '{}' step '{}' skipped", flow_name, step.name),
                        )
                        .await;
                    continue;
                }

                ctx.logger
                    .info(
                        ctx,
                        &format!("Flow '{}' step '{}' started", flow_name, step.name),
                    )
                    .await;

                match &step.action {
                    Action::Loop(arguments) => {
                        self.repeat(
                            ctx, flow, body, query, step, arguments, inference, tools, run,
                        )
                        .await?
                    }
                    _ => self
                        .step(ctx, flow, step, &values, inference, tools, run)
                        .await
                        .map_err(|err| failed(flow, step, err))?,
                }

                ctx.logger
                    .info(
                        ctx,
                        &format!("Flow '{}' step '{}' finished", flow_name, step.name),
                    )
                    .await;
            }
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat<'a>(
        &'a self,
        ctx: &'a AppContext,
        flow: &'a Definition,
        body: &'a Value,
        query: &'a Value,
        step: &'a Step,
        arguments: &'a ArgumentsLoop,
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let mut iterations = 0;
            loop {
                iterations += 1;
                self.steps(
                    ctx,
                    flow,
                    body,
                    query,
                    &arguments.steps,
                    inference,
                    tools,
                    run,
                )
                .await?;

                if let Some(until) = &arguments.until {
                    let values = values::template_values(ctx, flow, body, query, &run.steps);
                    if conditions::any(self.template_engine.as_ref(), ctx, until, &values)
                        .await
                        .map_err(|err| failed(flow, step, err))?
                    {
                        break;
                    }
                }

                if iterations >= arguments.max {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Flow '{}' step '{}' stopped after max {} iterations",
                                flow.metadata.name, step.name, arguments.max
                            ),
                        )
                        .await;
                    break;
                }
            }

            run.steps.insert(
                step.name.clone(),
                json!({ "output": iterations.to_string() }),
            );
            Ok(())
        })
    }
}
