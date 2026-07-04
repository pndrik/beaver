// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::skills::models::{Call, Schema, SkillPermission};

#[async_trait]
pub trait Skill {
    async fn name(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn description(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn schema(&self, ctx: &AppContext) -> Result<Schema, AppError>;
    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError>;
}
