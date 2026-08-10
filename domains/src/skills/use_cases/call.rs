// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{AgentPermissions, Conversation},
    },
    skills::{
        Skills,
        models::{Call, SkillPermission},
    },
};

async fn add_tool_message_to_conversation(
    _ctx: &AppContext,
    conversation: &mut Conversation,
    call: &Call,
    response_type: &str,
    response: &str,
) -> Result<(), AppError> {
    conversation.add_tool_message(format!(
        "Name: {}\nArguments: {}\n{}: \n{}",
        call.name,
        call.arguments_as_json(),
        response_type,
        response
    ));

    Ok(())
}

fn get_permission_for_skill(
    ctx: &AppContext,
    permissions: &AgentPermissions,
    skill_name: &str,
) -> Result<SkillPermission, AppError> {
    permissions
        .get_permission_for_skill(skill_name)
        .ok_or_else(|| {
            app_error!(
                Unauthorized,
                "skill_permission_denied",
                &format!(
                    "Agent does not have permission to call skill: {}",
                    skill_name
                ),
                ctx.clone()
            )
        })
}

impl Skills {
    pub async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        call: &Call,
    ) -> Result<(), AppError> {
        ctx.logger
            .trace(
                ctx,
                &format!(
                    "Agent '{}' is calling skill '{}'",
                    conversation.agent.metadata.name, call.name
                ),
            )
            .await;

        let permissions =
            match get_permission_for_skill(ctx, &conversation.agent.permissions, &call.name) {
                Ok(p) => p,
                Err(err) => {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Agent '{}' has no permission to access skill '{}'.",
                                conversation.agent.metadata.name, call.name
                            ),
                        )
                        .await;
                    add_tool_message_to_conversation(
                        ctx,
                        conversation,
                        call,
                        "Error",
                        &err.internal_message,
                    )
                    .await?;
                    return Ok(());
                }
            };

        for provider in &self.skills_providers {
            let skill = match provider.get(ctx, &call.name).await {
                Ok(s) => s,
                Err(_) => continue,
            };

            if !call.validate_arguments(skill.parameters) {
                add_tool_message_to_conversation(
                    ctx,
                    conversation,
                    call,
                    "Error",
                    "Arguments for skill are invalid please check schema",
                )
                .await?;
                return Ok(());
            }

            match provider.execute(ctx, permissions, call).await {
                Ok(res) => {
                    add_tool_message_to_conversation(ctx, conversation, call, "Result", &res)
                        .await?
                }
                Err(err) => {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Skill '{}' failed with: {}",
                                call.name, err.internal_message
                            ),
                        )
                        .await;
                    add_tool_message_to_conversation(
                        ctx,
                        conversation,
                        call,
                        "Error",
                        &err.internal_message,
                    )
                    .await?
                }
            };

            return Ok(());
        }

        ctx.logger
            .warn(ctx, &format!("Skill '{}' not found", call.name))
            .await;
        add_tool_message_to_conversation(
            ctx,
            conversation,
            call,
            "Error",
            &format!("Skill with name '{}' not found", call.name),
        )
        .await
    }

    pub async fn call_many(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        calls: Vec<Call>,
    ) -> Result<(), AppError> {
        for call in calls {
            self.call(ctx, conversation, &call).await?;
        }

        Ok(())
    }

    pub async fn call_many_with_subagent(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        inference: &Inference,
        calls: Vec<Call>,
    ) -> Result<(), AppError> {
        for call in calls {
            if call.name == "subagent" {
                let permissions =
                    get_permission_for_skill(ctx, &conversation.agent.permissions, &call.name)?;
                self.call_subagent_skill(ctx, conversation, permissions, &call, inference)
                    .await?;
                continue;
            }

            self.call(ctx, conversation, &call).await?;
        }

        Ok(())
    }
}
