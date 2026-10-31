// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::sync::Arc;

use app_domains::core::traits::{Configuration, Context};

#[derive(Debug)]
pub struct ContextStandard {
    trace_id: String,
    configuration: Arc<dyn Configuration + Send + Sync>,
}

impl ContextStandard {
    pub fn new(trace_id: String, configuration: Arc<dyn Configuration + Send + Sync>) -> Self {
        Self {
            trace_id,
            configuration,
        }
    }
}

impl Context for ContextStandard {
    fn trace_id(&self) -> &str {
        &self.trace_id
    }

    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync> {
        &self.configuration
    }

    fn template_values(&self) -> Map<String, Value> {
        Map::from_iter([("trace_id".to_string(), Value::from(self.trace_id.clone()))])
    }
}
