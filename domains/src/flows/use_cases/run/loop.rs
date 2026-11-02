// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};
use tracing::warn;

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
    pub(super) fn r#loop<'a>(
        &'a self,
        ctx: &'a AppContext,
        step: &'a Step,
        position: &'a str,
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let Action::Loop(arguments) = &step.action else {
                unreachable!("loop runs loop steps")
            };

            let mut iterations = 0;
            loop {
                iterations += 1;
                for (index, inner) in arguments.steps.iter().enumerate() {
                    let inner_position = Step::position(Some(position), index);
                    self.step(ctx, inner, &inner_position, inference, tools, run)
                        .await?;
                }

                let values = run.template_values(ctx);
                if arguments
                    .done(ctx, self.template_engine.as_ref(), &values)
                    .await
                    .map_err(|err| run.failed(step.label(position), err))?
                {
                    break;
                }

                if iterations >= arguments.max {
                    warn!(
                        flow = %run.flow.metadata.name,
                        step = %step.label(position),
                        max = arguments.max,
                        "Loop stopped after max iterations"
                    );
                    break;
                }
            }

            Ok(())
        })
    }
}
