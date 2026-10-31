// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, FlowRun, Step},
    },
    inference::{Inference, models::Message},
    tools::Tools,
};

impl Flows {
    pub(super) async fn agent(
        &self,
        ctx: &AppContext,
        step: &Step,
        inference: &Inference,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        let Action::Agent(arguments) = &step.action else {
            unreachable!("agent runs agent steps")
        };
        let values = run.template_values(ctx);
        let conversation = &mut run.context_mut(step).conversation;

        let prompt = self
            .template_engine
            .render(ctx, &arguments.prompt, &values)
            .await?;

        conversation.insert_message(Message::user(prompt, conversation.agent()), None);
        inference.infer(ctx, conversation, tools).await?;

        Ok(())
    }
}
