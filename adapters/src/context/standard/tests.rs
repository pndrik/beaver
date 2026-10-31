// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, sync::Arc};

use app_domains::core::{models::AppContext, traits::ContextFactory};

use super::{ContextFactoryStandard, ContextStandard};
use crate::{configuration::ConfigurationMap, logger::LoggerRecording};

fn parent() -> (AppContext, ContextFactoryStandard) {
    let logger = Arc::new(LoggerRecording::new());
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "parent".to_string(),
        Arc::new(ConfigurationMap::default()),
        logger.clone(),
    )));
    (ctx, ContextFactoryStandard::new(logger))
}

#[tokio::test]
async fn generates_unique_trace_ids() {
    let (parent, factory) = parent();
    let a = factory.create(&parent, None, HashMap::new()).await.unwrap();
    let b = factory.create(&parent, None, HashMap::new()).await.unwrap();

    assert!(!a.trace_id().is_empty());
    assert_ne!(a.trace_id(), "parent");
    assert_ne!(a.trace_id(), b.trace_id());
}

#[tokio::test]
async fn uses_given_trace_id_and_ignores_metadata() {
    let (parent, factory) = parent();
    let metadata = HashMap::from([("x-tenant".to_string(), "acme".to_string())]);
    let ctx = factory
        .create(&parent, Some("cli".to_string()), metadata)
        .await
        .unwrap();

    assert_eq!(ctx.trace_id(), "cli");
    assert_eq!(
        serde_json::to_value(&ctx).unwrap(),
        serde_json::json!({ "trace_id": "cli" })
    );
}
