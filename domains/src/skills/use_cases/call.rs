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

async fn get_permission_for_skill(
    ctx: &AppContext,
    agent_permissions: &AgentPermissions,
    skill_name: &str,
) -> Result<SkillPermission, AppError> {
    agent_permissions
        .skills
        .iter()
        .find(|p| p.name == skill_name)
        .cloned()
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

impl Skills {
    pub async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        call: &Call,
    ) -> Result<(), AppError> {
        let permissions =
            get_permission_for_skill(ctx, &conversation.agent.permissions, &call.name).await?;

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

        Err(app_error!(
            NotFound,
            "skill_not_found",
            &format!("Skill with name '{}' not found", call.name),
            ctx.clone()
        ))
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
                    get_permission_for_skill(ctx, &conversation.agent.permissions, &call.name)
                        .await?;

                self.call_subagent_skill(ctx, conversation, permissions, &call, inference)
                    .await?;
                continue;
            }

            self.call(ctx, conversation, &call).await?;
        }

        Ok(())
    }
}
