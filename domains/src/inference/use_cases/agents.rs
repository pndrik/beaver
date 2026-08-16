// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Agent},
};

impl Inference {
    pub async fn agents_list(&self, ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        self.agent_provider.list(ctx).await
    }
}
