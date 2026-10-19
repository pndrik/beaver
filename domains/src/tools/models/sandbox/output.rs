// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

/// What code in a sandbox produced.
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxOutput {
    /// Everything the code logged, one line per call.
    pub console: String,
    /// The result as JSON, `None` if there is none, or the error the code failed with.
    pub result: Result<Option<serde_json::Value>, String>,
}
