// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{Metadata, Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: Path,
    pub metadata: Metadata,
}
