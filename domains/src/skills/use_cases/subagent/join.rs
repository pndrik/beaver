// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{Arguments, MAX_SUBAGENT_ITERATIONS};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    skills::{
        Skills,
        models::{Schema, Skill, SkillPermission},
    },
};

pub(super) async fn join(
    ctx: &AppContext,
    conversation: &mut Conversation,
    permissions: SkillPermission,
    arguments: &Arguments,
    skills: &Skills,
    inference: &Inference,
) -> Result<(), AppError> {
    if !permissions.roles.contains(&"join".to_string()) {
        return Err(app_error!(
            Unauthorized,
            "skill_failed",
            "Can not join subagents without role 'join'.",
            ctx.clone()
        ));
    }

    let Some(name) = arguments.name.clone() else {
        return Err(app_error!(
            Validation,
            "skill_failed",
            "Missing required argument 'name' for action 'join'",
            ctx.clone()
        ));
    };

    let Some(prompt) = arguments.prompt.clone() else {
        return Err(app_error!(
            Validation,
            "skill_failed",
            "Missing required argument 'prompt' for action 'join'",
            ctx.clone()
        ));
    };

    if !permissions.scopes.contains(&name) {
        return Err(app_error!(
            Unauthorized,
            "skill_failed",
            &format!("Can not join subagent '{}' without scope '{}'.", name, name),
            ctx.clone()
        ));
    }

    conversation.add_assistant_message(
        conversation.agent.metadata.name.clone(),
        conversation.agent.metadata.display_name.clone(),
        prompt.clone(),
    );
    let subagent_skill = conversation
        .skills
        .iter()
        .find(|s| s.name == "subagent")
        .unwrap()
        .clone();
    conversation.skills = conversation
        .skills
        .iter()
        .filter(|s| s.name != "subagent")
        .cloned()
        .collect();
    conversation.skills.push(Skill {
        name: "subagent_leave".to_string(),
        description: "Leave the subagent conversation.".to_string(),
        parameters: Schema::new("subagent", ""),
    });

    let mut subagent_conversation = inference.new_conversation(ctx, &name, skills).await?;

    subagent_conversation.add_user_message(prompt);
    conversation.add_tool_message(
        "To end the conversation with the subagent call the 'subagent_leave' skill.".to_string(),
    );

    for _ in 0..MAX_SUBAGENT_ITERATIONS {
        inference
            .infer_no_subagent(ctx, &mut subagent_conversation, skills)
            .await?;
        conversation.add_message(subagent_conversation.get_latest_message().unwrap().clone());
        let leave = inference
            .infer_no_subagent(ctx, conversation, skills)
            .await?;
        let latest_message = conversation.get_latest_message_content().unwrap();
        if leave {
            conversation.skills = conversation
                .skills
                .iter()
                .filter(|s| s.name != "subagent_leave")
                .cloned()
                .collect();
            conversation.skills.push(subagent_skill);

            conversation.add_tool_message(format!(
                "The subagent '{}' has left the conversation.",
                name
            ));
            break;
        }

        subagent_conversation.add_user_message(latest_message);
    }

    Ok(())
}
