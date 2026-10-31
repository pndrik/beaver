// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::{fmt::Debug, sync::Arc};

use crate::core::{models::LogAttributes, traits::Configuration};

pub trait Context: Debug {
    fn trace_id(&self) -> &str;
    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync>;
    fn template_values(&self) -> Map<String, Value>;
    fn log_attributes(&self) -> LogAttributes {
        LogAttributes::new(self.trace_id())
    }
}
