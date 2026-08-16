// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::tools::provider::embedded::EmbeddedToolSet;
use serde::{Deserialize, Serialize};

mod configuration;
mod git_repository;
mod permissions;
mod tools;

const NAME: &str = "git";
const DESCRIPTION: &str = "A tool for interacting with Git repositories.";

#[derive(Deserialize, Serialize, Clone)]
struct Repository {
    url: String,
    email: String,
    display_name: Option<String>,
    username: Option<String>,
    ssh_key: Option<String>,
    password: Option<String>,
    #[serde(default)]
    known_hosts: Vec<String>,
}

pub struct Git;

impl Git {
    pub fn new() -> EmbeddedToolSet {
        EmbeddedToolSet::new(
            NAME,
            DESCRIPTION,
            vec![
                Box::new(tools::Clone),
                Box::new(tools::Pull),
                Box::new(tools::Checkout),
                Box::new(tools::Commit),
                Box::new(tools::Push),
                Box::new(tools::Diff),
                Box::new(tools::BranchDelete),
                Box::new(tools::Branches),
            ],
        )
    }
}
