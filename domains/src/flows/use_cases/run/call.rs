// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};

use serde_json::{Map, Value};

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
    pub(super) fn call<'a>(
        &'a self,
        ctx: &'a AppContext,
        step: &'a Step,
        inference: &'a Inference,
        tools: &'a Tools,
        run: &'a mut FlowRun,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        Box::pin(async move {
            let Action::Flow(arguments) = &step.action else {
                unreachable!("call runs flow steps")
            };
            let flow = self.flow_provider.get(ctx, &arguments.flow).await?;
            flow.validate(ctx)?;

            let values = run.template_values(ctx);
            let mut raw = Map::new();
            for (name, template) in &arguments.arguments {
                let rendered = self.template_engine.render(ctx, template, &values).await?;
                raw.insert(name.clone(), Value::String(rendered));
            }

            let mut child = run.child(ctx, flow, &raw)?;
            self.execute(ctx, &mut child, inference, tools).await?;
            run.called(step, child);
            Ok(())
        })
    }
}
