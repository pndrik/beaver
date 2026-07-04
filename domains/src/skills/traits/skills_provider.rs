// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Arc;

use crate::core::models::{AppContext, AppError};
use crate::skills::models::{Call, Skill, SkillPermission};

#[async_trait]
pub trait SkillsProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError>;
    async fn add_skill(
        &mut self,
        ctx: &AppContext,
        skill: Arc<dyn super::Skill + Send + Sync>,
    ) -> Result<(), AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Skill>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Skill, AppError>;
    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError>;
}
