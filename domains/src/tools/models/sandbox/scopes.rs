// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{SandboxFetchScope, SandboxFilesystemOperation, SandboxFilesystemScope};

/// What code in a sandbox may access besides its chroot's modules. Nothing
/// by default: without fetch scopes there is no web access, without
/// filesystem scopes no file access.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SandboxScopes {
    #[serde(default)]
    pub fetch: Vec<SandboxFetchScope>,
    #[serde(default)]
    pub filesystem: Vec<SandboxFilesystemScope>,
}

impl SandboxScopes {
    pub fn allows_url(&self, url: &str) -> bool {
        self.fetch.iter().any(|scope| scope.allows_url(url))
    }

    pub fn allows_fetch(&self, url: &str, method: &str) -> bool {
        self.fetch.iter().any(|scope| scope.allows(url, method))
    }

    pub fn allows_file(&self, path: &str, operation: SandboxFilesystemOperation) -> bool {
        self.filesystem
            .iter()
            .any(|scope| scope.allows(path, operation))
    }
}
