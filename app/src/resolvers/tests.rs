// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{configuration::ConfigurationMap, context::ContextStandard};
use app_domains::core::models::AppContext;

use super::logging;

fn context(provider: &str) -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(
            [("logger.provider", provider)]
                .into_iter()
                .collect::<ConfigurationMap>(),
        ),
    )))
}

#[tokio::test]
async fn resolves_the_stdout_logger() {
    assert!(logging(&context("stdout")).await.is_ok());
}

#[tokio::test]
async fn rejects_unknown_logger_providers() {
    let err = logging(&context("syslog")).await.err().unwrap();
    assert_eq!(err.translation_key, "bootstrap_failed");
    assert!(err.internal_message.contains("syslog"));
}
