// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::fmt::Debug;
use tracing::Span;

use crate::core::models::{AppContext, AppError};

pub trait Logging: Debug {
    fn init(&self, ctx: &AppContext, level: &str) -> Result<(), AppError>;
    fn context_span(&self, ctx: &AppContext) -> Span;
}
