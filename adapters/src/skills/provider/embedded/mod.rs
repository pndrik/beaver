// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Arc;

use crate::skills::embedded::File;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::models::Call,
    skills::models::SkillPermission,
    skills::traits::{Skill, SkillsProvider},
};

pub struct EmbeddedProvider {
    skills: Vec<Arc<dyn Skill + Send + Sync>>,
}

impl EmbeddedProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            skills: vec![Arc::new(File {})],
        }
    }
}

#[async_trait]
impl SkillsProvider for EmbeddedProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn add_skill(
        &mut self,
        ctx: &AppContext,
        skill: Arc<dyn Skill + Send + Sync>,
    ) -> Result<(), AppError> {
        let skill_name = skill.name(ctx).await?;
        for existing_skill in &self.skills {
            if existing_skill.name(ctx).await? == skill_name {
                return Err(app_error!(
                    Conflict,
                    "skill_already_exists",
                    &format!("Skill with name '{}' already exists", skill_name),
                    ctx.clone()
                ));
            }
        }
        self.skills.push(Arc::from(skill));
        Ok(())
    }

    async fn list(
        &self,
        _ctx: &AppContext,
    ) -> Result<Vec<app_domains::skills::models::Skill>, AppError> {
        let mut skills = Vec::new();
        for skill in &self.skills {
            skills.push(app_domains::skills::models::Skill {
                name: skill.name(_ctx).await?,
                description: skill.description(_ctx).await?,
                parameters: skill.schema(_ctx).await?,
            });
        }
        Ok(skills)
    }

    async fn get(
        &self,
        ctx: &AppContext,
        name: &str,
    ) -> Result<app_domains::skills::models::Skill, AppError> {
        for skill in &self.skills {
            let skill_name = skill.name(ctx).await?;
            if skill_name == name {
                return Ok(app_domains::skills::models::Skill {
                    name: skill_name,
                    description: skill.description(ctx).await?,
                    parameters: skill.schema(ctx).await?,
                });
            }
        }

        Err(app_error!(
            NotFound,
            "skill_not_found",
            &format!("Skill with name '{}' not found", name),
            ctx.clone()
        ))
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        for skill in &self.skills {
            let skill_name = skill.name(ctx).await?;
            if skill_name == input.name {
                return skill.execute(ctx, permissions, &input).await;
            }
        }

        Err(app_error!(
            NotFound,
            "skill_not_found",
            &format!("Skill with name '{}' not found", input.name),
            ctx.clone()
        ))
    }
}
