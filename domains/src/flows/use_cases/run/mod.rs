// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{ArgumentsAgent, Definition},
    },
    inference::{
        Inference,
        models::{Conversation, Message},
    },
    tools::Tools,
};

mod chroot;
mod conditions;
mod context;
mod step;
mod steps;
mod tool;
mod values;

/// State of a named context within one flow run.
struct RunContext {
    chroot: PathBuf,
    conversation: Conversation,
}

/// State of one flow run. Every trigger gets its own, nothing is shared.
#[derive(Default)]
struct FlowRun {
    contexts: HashMap<String, RunContext>,
    steps: Map<String, Value>,
    delete_on_finish: Vec<PathBuf>,
}

impl Flows {
    pub async fn run(
        &self,
        ctx: &AppContext,
        flow: &Definition,
        body: &Value,
        query: &Value,
        inference: &Inference,
        tools: &Tools,
    ) -> Result<(), AppError> {
        let flow_name = &flow.metadata.name;
        ctx.logger
            .info(ctx, &format!("Flow '{}' started", flow_name))
            .await;

        flow.validate(ctx)?;

        let mut run = FlowRun::default();
        let result = self
            .steps(
                ctx,
                flow,
                body,
                query,
                &flow.spec.steps,
                inference,
                tools,
                &mut run,
            )
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
    }

    async fn agent(
        &self,
        ctx: &AppContext,
        context: &mut RunContext,
        arguments: &ArgumentsAgent,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let conversation = &mut context.conversation;
        let agent = conversation.agent().metadata.name.clone();

        let prompt = self
            .template_engine
            .render(ctx, &arguments.prompt, values)
            .await?;
        ctx.logger
            .trace(ctx, &format!("[prompt → {}]: {}", agent, prompt))
            .await;

        conversation.insert_message(Message::user(prompt, conversation.agent()), None);
        inference.infer(ctx, conversation, tools).await?;

        let output = conversation
            .last()
            .and_then(|message| message.as_agent())
            .and_then(|message| message.content.clone())
            .unwrap_or_default();
        ctx.logger
            .trace(ctx, &format!("[{}]: {}", agent, output))
            .await;

        Ok(output)
    }
}
