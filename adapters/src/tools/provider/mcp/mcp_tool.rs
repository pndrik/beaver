// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::tools::models::Tool;

pub(super) struct McpTool {
    pub server: String,
    pub tool: Tool,
}

impl McpTool {
    pub fn new(server: String, tool: Tool) -> Self {
        Self { server, tool }
    }
}
