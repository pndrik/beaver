// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::Serialize;
use std::sync::Arc;

use crate::core::traits::{Configuration, Logger};

#[derive(Debug, Clone, Serialize)]
pub struct AppContext {
    pub trace_id: String,

    #[serde(skip_serializing)]
    pub configuration: Arc<dyn Configuration + Send + Sync>,
    #[serde(skip_serializing)]
    pub logger: Arc<dyn Logger + Send + Sync>,
}

impl AppContext {
    pub fn new(
        trace_id: String,
        configuration: Arc<dyn Configuration + Send + Sync>,
        logger: Arc<dyn Logger + Send + Sync>,
    ) -> Self {
        Self {
            trace_id,
            configuration,
            logger,
        }
    }
}
