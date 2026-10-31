// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Serialize, Serializer};
use std::sync::Arc;

use super::LogAttributes;
use crate::core::traits::{Configuration, Context};

/// Cheap-clone handle over a `Context` implementation.
#[derive(Debug, Clone)]
pub struct AppContext(Arc<dyn Context + Send + Sync>);

impl AppContext {
    pub fn new(inner: Arc<dyn Context + Send + Sync>) -> Self {
        Self(inner)
    }

    pub fn trace_id(&self) -> &str {
        self.0.trace_id()
    }

    pub fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync> {
        self.0.configuration()
    }

    pub fn log_attributes(&self) -> LogAttributes {
        self.0.log_attributes()
    }
}

impl Serialize for AppContext {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.template_values().serialize(serializer)
    }
}
