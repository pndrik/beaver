use std::sync::Arc;

use crate::core::models::AppError;

pub mod models;
pub mod traits;
pub mod use_cases;

pub struct Skills {
    pub skills_providers: Vec<Arc<dyn traits::SkillsProvider + Send + Sync>>,
}

impl Skills {
    pub fn new(
        skills_providers: Vec<Arc<dyn traits::SkillsProvider + Send + Sync>>,
    ) -> Result<Self, AppError> {
        Ok(Self { skills_providers })
    }
}
