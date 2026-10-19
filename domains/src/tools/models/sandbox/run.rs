// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{SandboxLimits, SandboxScopes, SandboxSource};

/// A request to run code in a [`crate::tools::traits::CodeSandbox`].
#[derive(Debug, Clone)]
pub struct SandboxRun {
    pub source: SandboxSource,
    pub scopes: SandboxScopes,
    pub limits: Option<SandboxLimits>,
}
