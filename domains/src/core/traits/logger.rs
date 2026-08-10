// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::fmt::Debug;

use crate::core::models::AppContext;

#[async_trait]
pub trait Logger: Debug {
    async fn trace(&self, ctx: &AppContext, message: &str);
    async fn debug(&self, ctx: &AppContext, message: &str);
    async fn info(&self, ctx: &AppContext, message: &str);
    async fn warn(&self, ctx: &AppContext, message: &str);
    async fn error(&self, ctx: &AppContext, message: &str);
    async fn fatal(&self, ctx: &AppContext, message: &str);
}
