// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::SandboxFilesystemOperation;
use crate::core::models::LiteralOrRegex;

/// Allows `operations` on `path`, an absolute path in the chroot. A literal
/// path also covers everything below it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SandboxFilesystemScope {
    pub path: LiteralOrRegex,
    pub operations: Vec<SandboxFilesystemOperation>,
}

impl SandboxFilesystemScope {
    pub fn allows(&self, path: &str, operation: SandboxFilesystemOperation) -> bool {
        self.operations.contains(&operation) && self.covers(path)
    }

    fn covers(&self, path: &str) -> bool {
        match &self.path {
            LiteralOrRegex::Literal(literal) => {
                path == literal
                    || path
                        .strip_prefix(literal.trim_end_matches('/'))
                        .is_some_and(|rest| rest.starts_with('/'))
            }
            LiteralOrRegex::Regex(regex) => regex.is_match(path),
        }
    }
}
