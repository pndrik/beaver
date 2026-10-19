// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::tools::provider::embedded::EmbeddedToolSet;
use app_domains::tools::traits::CodeSandbox;

mod permissions;
mod tools;

#[cfg(test)]
mod tests;

const NAME: &str = "javascript";
const DESCRIPTION: &str = "A tool for running JavaScript over files in the chroot.";

pub struct JavaScript;

impl JavaScript {
    pub fn new(sandbox: Arc<dyn CodeSandbox + Send + Sync>) -> EmbeddedToolSet {
        EmbeddedToolSet::new(NAME, DESCRIPTION, vec![Box::new(tools::Run::new(sandbox))])
    }
}
