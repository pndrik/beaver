// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::skills::models::Skill;

pub(super) struct McpSkill {
    pub server: String,
    pub skill: Skill,
}

impl McpSkill {
    pub fn new(server: String, skill: Skill) -> Self {
        Self { server, skill }
    }
}
