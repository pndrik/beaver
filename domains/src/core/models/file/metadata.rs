// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::Kind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub kind: Kind,
    pub size: u64,
    pub modified: u64,
}

impl Metadata {
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }
}
