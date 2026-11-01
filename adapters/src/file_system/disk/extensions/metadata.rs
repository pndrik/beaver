// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{io, time::UNIX_EPOCH};

use cap_std::fs::Metadata;

use app_domains::core::models::file;

pub(crate) trait AsBeaverFileMetadata {
    fn as_beaver_file_metadata(&self) -> io::Result<file::Metadata>;
}
impl AsBeaverFileMetadata for Metadata {
    fn as_beaver_file_metadata(&self) -> io::Result<file::Metadata> {
        let file_type = self.file_type();
        let kind = if file_type.is_symlink() {
            file::Kind::Symlink
        } else if file_type.is_dir() {
            file::Kind::Directory
        } else if file_type.is_file() {
            file::Kind::File
        } else {
            file::Kind::Other
        };

        Ok(file::Metadata {
            kind,
            size: self.len(),
            modified: self
                .modified()?
                .into_std()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
        })
    }
}
