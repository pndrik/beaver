// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::PathBuf;

use app_domains::core::models::file;

pub(crate) trait AsRelativePathBuf {
    fn as_relative_path_buf(&self) -> PathBuf;
}
impl AsRelativePathBuf for file::Path {
    fn as_relative_path_buf(&self) -> PathBuf {
        if self.is_root() {
            return PathBuf::from(".");
        }

        PathBuf::from(&self.as_str()[1..])
    }
}
