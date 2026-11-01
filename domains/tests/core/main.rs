// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod chroot;
mod file_path;

use std::sync::Arc;

use app_adapters::{configuration::ConfigurationMap, context::ContextStandard};
use app_domains::core::models::AppContext;

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}
