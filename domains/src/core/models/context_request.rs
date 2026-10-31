// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ContextRequest {
    /// `None` lets the factory generate a trace id.
    pub trace_id: Option<String>,
    /// Request data such as HTTP headers. Header names are lowercase.
    pub metadata: HashMap<String, String>,
}
