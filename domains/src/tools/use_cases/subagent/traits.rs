// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Schema, ToolPermission},
    },
};

#[async_trait]
pub(super) trait SubagentTool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn schema(&self, ctx: &AppContext) -> Result<Schema, AppError>;
    async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
        tools: &Tools,
        inference: &Inference,
    ) -> Result<(), AppError>;
}
