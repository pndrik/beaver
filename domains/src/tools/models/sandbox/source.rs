// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

/// What a code sandbox runs.
#[derive(Debug, Clone, PartialEq)]
pub enum SandboxSource {
    /// Code run as is, imports resolve inside the chroot. The result is its
    /// return value.
    Code(String),
    /// A file at an absolute path in the chroot, run as module. The result is
    /// its default export, called if it is a function.
    File(String),
    /// Calls the exported function `name` of the module at `path` in the host
    /// `directory` with `arguments`, e.g. a tool. Its imports resolve inside
    /// `directory`. The result is its return value.
    Function {
        directory: String,
        path: String,
        name: String,
        arguments: Vec<serde_json::Value>,
    },
}
