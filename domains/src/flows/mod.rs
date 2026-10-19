// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::core::{models::AppError, traits::TemplateEngine};

pub(crate) mod helpers;
pub mod models;
pub mod traits;
pub mod use_cases;

pub struct Flows {
    pub(crate) flow_provider: Arc<dyn traits::FlowProvider + Send + Sync>,
    pub(crate) template_engine: Arc<dyn TemplateEngine + Send + Sync>,
}

impl Flows {
    pub fn new(
        flow_provider: Arc<dyn traits::FlowProvider + Send + Sync>,
        template_engine: Arc<dyn TemplateEngine + Send + Sync>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            flow_provider,
            template_engine,
        })
    }
}
