// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::tools::provider::embedded::EmbeddedToolSet;

mod permissions;
mod tools;

const NAME: &str = "file";
const DESCRIPTION: &str = "A tool for reading/writing files and listing directories.";

pub struct File;

impl File {
    pub fn new() -> EmbeddedToolSet {
        EmbeddedToolSet::new(
            NAME,
            DESCRIPTION,
            vec![
                Box::new(tools::Delete),
                Box::new(tools::Find),
                Box::new(tools::Mkdir),
                Box::new(tools::Read),
                Box::new(tools::Search),
                Box::new(tools::Write),
            ],
        )
    }
}
