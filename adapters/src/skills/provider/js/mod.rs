// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::{
        models::{Call, SkillPermission},
        traits::{Skill, SkillsProvider},
    },
};

mod javascript;
use javascript::{JavaScript, Value};
mod file;
mod helper;
mod models;

const CONFIGURATION_SKILLS_DIR: &str = "skills.providers.javascript.path";
const CONFIGURATION_SKILL_CONFIGURATION: &str = "skills.configuration";

pub struct JavascriptProvider {
    skills: HashMap<String, models::Skill>,
}

impl JavascriptProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
}

#[async_trait]
impl SkillsProvider for JavascriptProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let dirs = file::list_directories(ctx, &self.get_skills_path(ctx).await?)?;
        self.skills.clear();

        for dir in dirs {
            let package = match self.load_package_info(ctx, &dir).await {
                Ok(pkg) => pkg,
                Err(_) => {
                    continue;
                }
            };

            for (name, skill) in package.config.beaver.skills {
                self.skills.insert(
                    name.clone(),
                    models::Skill {
                        directory: dir.clone(),
                        package: package.name.clone(),
                        main: skill.main.clone(),
                        description: skill.description.clone(),
                        parameters: skill.parameters.clone(),
                        scopes: skill.scopes.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    async fn add_skill(
        &mut self,
        ctx: &AppContext,
        _skill: Arc<dyn Skill + Send + Sync>,
    ) -> Result<(), AppError> {
        Err(app_error!(
            Conflict,
            "denied",
            "Can not programmatically add skills to the JavascriptProvider",
            ctx.clone()
        ))
    }

    async fn list(
        &self,
        _ctx: &AppContext,
    ) -> Result<Vec<app_domains::skills::models::Skill>, AppError> {
        self.skills
            .iter()
            .map(|(name, skill)| {
                Ok(app_domains::skills::models::Skill {
                    name: format!("js_{}", name),
                    description: skill.description.clone(),
                    parameters: skill.parameters.clone(),
                })
            })
            .collect::<Result<Vec<app_domains::skills::models::Skill>, AppError>>()
    }

    async fn get(
        &self,
        ctx: &AppContext,
        name: &str,
    ) -> Result<app_domains::skills::models::Skill, AppError> {
        let name = name.strip_prefix("js_").unwrap_or(name);
        let Some(skill) = self.skills.get(name) else {
            return Err(app_error!(
                NotFound,
                "skill_not_found",
                &format!("Skill with name '{}' not found", name),
                ctx.clone()
            ));
        };

        Ok(app_domains::skills::models::Skill {
            name: format!("js_{}", name),
            description: skill.description.clone(),
            parameters: skill.parameters.clone(),
        })
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let name = input.name.strip_prefix("js_").unwrap_or(&input.name);
        let Some(skill) = self.skills.get(name) else {
            return Err(app_error!(
                NotFound,
                "skill_not_found",
                &format!("Skill with name '{}' not found", name),
                ctx.clone()
            ));
        };

        // TBD: This has room for improvement
        let configuration = self.get_skill_configuration(ctx, &skill.package).await?;
        let input_value = self.input_to_value(&input);
        let permissions_value = self.permission_to_value(&permissions);
        let ctx_clone = ctx.clone();
        let directory = skill.directory.clone();
        let main = skill.main.clone();
        let scopes = skill.scopes.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut js =
                JavaScript::new(directory.clone(), format!("{}/{}", directory, main), scopes);

            js.call(
                &ctx_clone,
                "main",
                vec![input_value, configuration, permissions_value],
            )
        })
        .await
        .map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Skill '{}' execution failed: {}", input.name, e),
                ctx.clone()
            )
        })??;

        Ok(match result {
            Value::String(s) => s,
            _ => {
                return Err(app_error!(
                    Internal,
                    "skill_failed",
                    &format!(
                        "Skill '{}' execution returned non-string result.",
                        input.name
                    ),
                    ctx.clone()
                ));
            }
        })
    }
}
