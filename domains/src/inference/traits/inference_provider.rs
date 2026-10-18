// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::inference::models::{Conversation, Model, Options};

#[async_trait]
pub trait InferenceProvider {
    /// Appends the response as an agent message, including its tool calls.
    async fn infer(
        &self,
        ctx: &AppContext,
        options: &Options,
        conversation: &mut Conversation,
    ) -> Result<(), AppError>;
    async fn supported_models(&self, ctx: &AppContext) -> Result<Vec<Model>, AppError>;
    async fn supports_model(&self, ctx: &AppContext, model: &Model) -> Result<bool, AppError>;
}
