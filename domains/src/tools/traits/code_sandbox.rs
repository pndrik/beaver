// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::{
    core::models::{AppContext, AppError},
    tools::models::{SandboxOutput, SandboxRun},
};

#[async_trait]
pub trait CodeSandbox {
    async fn run(
        &self,
        ctx: &AppContext,
        chroot: &str,
        run: SandboxRun,
    ) -> Result<SandboxOutput, AppError>;
}
