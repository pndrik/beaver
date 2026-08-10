// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    skills::{Skills, models::Skill},
};

impl Skills {
    pub async fn list_all(&self, ctx: &AppContext) -> Result<Vec<Skill>, AppError> {
        let mut skills_found = Vec::new();
        for provider in &self.skills_providers {
            skills_found.extend(provider.list(ctx).await?);
        }

        Ok(skills_found)
    }
}
