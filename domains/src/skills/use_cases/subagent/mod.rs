// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    skills::{
        Skills,
        models::{Call, SkillPermission},
    },
};

pub const NAME: &str = "subagent";
pub const DESCRIPTION: &str = "A skill for talking to subagents.";
const MAX_SUBAGENT_ITERATIONS: usize = 20;

mod arguments;
use arguments::*;

mod join;
use join::*;

mod list;
use list::*;

mod schema;
pub(crate) use schema::*;

impl Skills {
    pub async fn call_subagent_skill(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: SkillPermission,
        input: &Call,
        inference: &Inference,
    ) -> Result<(), AppError> {
        let arguments = skill_call_to_arguments(ctx, input)?;

        match arguments.action.as_str() {
            "list" => {
                list(
                    ctx,
                    conversation,
                    permissions,
                    inference.agent_provider.clone(),
                )
                .await
            }
            "join" => join(ctx, conversation, permissions, &arguments, self, inference).await,
            other => Err(app_error!(
                Validation,
                "invalid_action",
                &format!("Invalid action: {}", other),
                ctx.clone()
            )),
        }
    }
}
