// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};

use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Definition, FlowRun},
    },
    inference::Inference,
    tools::Tools,
};

mod agent;
mod call;
mod chroot;
mod context;
mod execute;
mod r#loop;
mod step;
mod tool;
mod values;

impl Flows {
    pub async fn run(
        &self,
        ctx: &AppContext,
        flow: &Definition,
        arguments: &Map<String, Value>,
        inference: &Inference,
        tools: &Tools,
    ) -> Result<(), AppError> {
        flow.validate(ctx)?;
        let arguments = flow.arguments(ctx, arguments)?;
        let mut run = FlowRun::new(flow.clone(), arguments, 0);
        self.execute(ctx, &mut run, inference, tools).await
    }
}
