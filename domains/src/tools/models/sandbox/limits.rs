// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

/// Limits after which a sandbox stops the code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SandboxLimits {
    pub loop_iterations: u64,
    pub recursion: usize,
}
