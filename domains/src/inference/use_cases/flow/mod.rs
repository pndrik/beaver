// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, path::PathBuf};

use serde_json::{Map, Value, json};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{
            Conversation, Flow, FlowAction, FlowAgentArguments, FlowChroot, FlowStep,
            FlowToolArguments,
        },
    },
    tools::{
        Tools,
        models::{Call, CallValue, ToolPermission},
    },
};

mod chroot;
mod values;

/// State of a named context within one flow run.
struct RunContext {
    app_ctx: AppContext,
    chroot: PathBuf,
    declared_chroot: Option<FlowChroot>,
    conversation: Option<Conversation>,
}

/// State of one flow run. Every trigger gets its own, nothing is shared.
#[derive(Default)]
struct FlowRun {
    contexts: HashMap<String, RunContext>,
    steps: Map<String, Value>,
    delete_on_finish: Vec<PathBuf>,
}

impl Inference {
    pub async fn flow_run(
        &self,
        ctx: &AppContext,
        flow: &Flow,
        body: &Value,
        query: &Value,
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
                .flow_step(ctx, flow, step, &values, tools, &mut run)
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

    async fn flow_step(
        &self,
        ctx: &AppContext,
        flow: &Flow,
        step: &FlowStep,
        values: &Value,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        match run.contexts.get(&step.context.name) {
            None => {
                let context = self
                    .flow_context(ctx, step, values, &mut run.delete_on_finish)
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
            FlowAction::Agent(arguments) => {
                self.flow_agent(context, arguments, values, tools).await?
            }
            FlowAction::Tool(arguments) => {
                self.flow_tool(context, arguments, values, tools).await?
            }
        };

        let global = chroot::global_chroot(ctx)?;
        let messages = context
            .conversation
            .as_ref()
            .map_or(json!([]), |c| values::messages(c.messages()));
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

    async fn flow_context(
        &self,
        ctx: &AppContext,
        step: &FlowStep,
        values: &Value,
        delete_on_finish: &mut Vec<PathBuf>,
    ) -> Result<RunContext, AppError> {
        let chroot = match &step.context.chroot {
            Some(declared) => {
                let path = self
                    .template_engine
                    .render(ctx, &declared.path, values)
                    .await?;
                let chroot =
                    chroot::prepare(ctx, &path, declared.create_if_missing, declared.delete)?;
                if declared.delete && !delete_on_finish.contains(&chroot) {
                    delete_on_finish.push(chroot.clone());
                }
                chroot
            }
            None => chroot::global_chroot(ctx)?,
        };

        let app_ctx = AppContext::new(
            ctx.trace_id.clone(),
            chroot.to_string_lossy().to_string(),
            ctx.configuration.clone(),
            ctx.logger.clone(),
        )?;

        Ok(RunContext {
            app_ctx,
            chroot,
            declared_chroot: step.context.chroot.clone(),
            conversation: None,
        })
    }

    async fn flow_agent(
        &self,
        context: &mut RunContext,
        arguments: &FlowAgentArguments,
        values: &Value,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let ctx = &context.app_ctx;
        if let Some(conversation) = &context.conversation
            && conversation.agent.metadata.name != arguments.agent
        {
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

        let conversation = match &mut context.conversation {
            Some(conversation) => conversation,
            None => context
                .conversation
                .insert(self.new_conversation(ctx, &arguments.agent, tools).await?),
        };

        conversation.add_user_message(prompt);
        self.infer(ctx, conversation, tools).await?;

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

    async fn flow_tool(
        &self,
        context: &mut RunContext,
        arguments: &FlowToolArguments,
        values: &Value,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let ctx = &context.app_ctx;
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
        let output = tools.execute_direct(ctx, permissions, &call).await?;

        if let Some(conversation) = &mut context.conversation {
            conversation.add_tool_message(format!(
                "Name: {}\nArguments: {}\nResult: \n{}",
                call.name,
                call.arguments_as_json(),
                output
            ));
        }

        Ok(output)
    }
}
