// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug};

use crate::core::models::{AppContext, AppError};

#[async_trait]
pub trait Configuration: Debug {
    async fn get_string(&self, ctx: &AppContext, key: &str) -> Result<String, AppError>;
    async fn get_int(&self, ctx: &AppContext, key: &str) -> Result<i64, AppError>;
    async fn get_bool(&self, ctx: &AppContext, key: &str) -> Result<bool, AppError>;
    async fn get_map(
        &self,
        ctx: &AppContext,
        key: &str,
    ) -> Result<HashMap<String, String>, AppError>;
}
