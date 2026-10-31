// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::{fmt::Debug, sync::Arc};

use crate::core::traits::{Configuration, Logger};

pub trait Context: Debug {
    fn trace_id(&self) -> &str;
    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync>;
    fn logger(&self) -> &Arc<dyn Logger + Send + Sync>;
    fn template_values(&self) -> Map<String, Value>;
}
