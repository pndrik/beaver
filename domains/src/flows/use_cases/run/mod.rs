// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, path::PathBuf};

use serde_json::{Map, Value, json};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, AgentArguments, Chroot, Definition, Step, ToolArguments},
    },
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, CallValue, ToolPermission},
    },
};

mod chroot;
mod values;

/// State of a named context within one flow run.
struct RunContext {
    chroot: PathBuf,
    declared_chroot: Option<Chroot>,
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
        let mut result = Ok(());
        for step in &flow.spec.steps {
            ctx.logger
                .info(
                    ctx,
                    &format!("Flow '{}' step '{}' started", flow_name, step.name),
                )
                .await;

            let values = values::template_values(ctx, flow, body, query, &run.steps);
            if let Err(mut err) = self
                .step(ctx, flow, step, &values, inference, tools, &mut run)
                .await
            {
                err.internal_message = format!(
                    "Flow '{}' step '{}' failed: {}",
                    flow_name, step.name, err.internal_message
                );
                result = Err(err);
                break;
            }

            ctx.logger
                .info(
                    ctx,
                    &format!("Flow '{}' step '{}' finished", flow_name, step.name),
                )
                .await;
        }

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

    #[allow(clippy::too_many_arguments)]
    async fn step(
        &self,
        ctx: &AppContext,
        flow: &Definition,
        step: &Step,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        match run.contexts.get(&step.context.name) {
            None => {
                let context = self
                    .context(
                        ctx,
                        flow,
                        step,
                        values,
                        inference,
                        tools,
                        &mut run.delete_on_finish,
                    )
                    .await?;
                run.contexts.insert(step.context.name.clone(), context);
            }
            Some(context) => {
                if step.context.chroot.is_some() && step.context.chroot != context.declared_chroot {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Flow '{}' step '{}' declares a different chroot for context '{}'; keeping the first one",
                                flow.metadata.name, step.name, step.context.name
                            ),
                        )
                        .await;
                }
            }
        }

        let context = run
            .contexts
            .get_mut(&step.context.name)
            .expect("context was just created");

        let output = match &step.action {
            Action::Agent(arguments) => {
                self.agent(ctx, context, arguments, values, inference, tools)
                    .await?
            }
            Action::Tool(arguments) => self.tool(ctx, context, arguments, values, tools).await?,
        };

        let global = chroot::global_chroot(ctx).await?;
        let messages = values::messages(context.conversation.messages());
        run.steps.insert(
            step.name.clone(),
            json!({
                "output": output,
                "context": step.context.name,
                "chroot": chroot::relative(&global, &context.chroot),
                "messages": messages,
            }),
        );

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn context(
        &self,
        ctx: &AppContext,
        flow: &Definition,
        step: &Step,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
        delete_on_finish: &mut Vec<PathBuf>,
    ) -> Result<RunContext, AppError> {
        let chroot = match &step.context.chroot {
            Some(declared) => {
                let path = self
                    .template_engine
                    .render(ctx, &declared.path, values)
                    .await?;
                let chroot =
                    chroot::prepare(ctx, &path, declared.create_if_missing, declared.delete)
                        .await?;
                if declared.delete && !delete_on_finish.contains(&chroot) {
                    delete_on_finish.push(chroot.clone());
                }
                chroot
            }
            None => chroot::global_chroot(ctx).await?,
        };

        let agent = flow.context_agent(&step.context.name).ok_or_else(|| {
            app_error!(
                Validation,
                "flow_invalid",
                &format!("Context '{}' has no agent step", step.context.name),
                ctx.clone()
            )
        })?;
        let conversation = inference
            .new_conversation(ctx, agent, tools, &chroot.to_string_lossy())
            .await?;

        Ok(RunContext {
            chroot,
            declared_chroot: step.context.chroot.clone(),
            conversation,
        })
    }

    async fn agent(
        &self,
        ctx: &AppContext,
        context: &mut RunContext,
        arguments: &AgentArguments,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let conversation = &mut context.conversation;
        if conversation.agent.metadata.name != arguments.agent {
            return Err(app_error!(
                Validation,
                "flow_invalid",
                &format!(
                    "Context belongs to agent '{}' and can't be used by agent '{}'",
                    conversation.agent.metadata.name, arguments.agent
                ),
                ctx.clone()
            ));
        }

        let prompt = self
            .template_engine
            .render(ctx, &arguments.prompt, values)
            .await?;
        ctx.logger
            .trace(ctx, &format!("[prompt → {}]: {}", arguments.agent, prompt))
            .await;

        conversation.add_user_message(prompt);
        inference.infer(ctx, conversation, tools).await?;

        let output = conversation
            .messages()
            .iter()
            .rev()
            .find(|message| message.is_assistant())
            .map(|message| message.content.clone())
            .unwrap_or_default();
        ctx.logger
            .trace(ctx, &format!("[{}]: {}", arguments.agent, output))
            .await;

        Ok(output)
    }

    async fn tool(
        &self,
        ctx: &AppContext,
        context: &mut RunContext,
        arguments: &ToolArguments,
        values: &Value,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let rendered = values::render_arguments(
            self.template_engine.as_ref(),
            ctx,
            &arguments.arguments,
            values,
        )
        .await?;
        let call_arguments: HashMap<String, CallValue> = match rendered {
            Value::Null => HashMap::new(),
            rendered => serde_json::from_value(rendered).map_err(|e| {
                app_error!(
                    Validation,
                    "tool_arguments_invalid",
                    &format!("Invalid arguments for tool '{}': {}", arguments.tool, e),
                    ctx.clone()
                )
            })?,
        };

        let call = Call::new(arguments.tool.clone(), call_arguments);
        let permissions = ToolPermission {
            name: arguments.tool.clone(),
            scopes: arguments.scopes.clone(),
        };
        let output = tools
            .execute_direct(ctx, &mut context.conversation, permissions, &call)
            .await?;

        context.conversation.add_tool_message(format!(
            "Name: {}\nArguments: {}\nResult: \n{}",
            call.name,
            call.arguments_as_json(),
            output
        ));

        Ok(output)
    }
}
