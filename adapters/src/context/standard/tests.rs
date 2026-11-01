// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::core::{
    models::{AppContext, LogAttributes},
    traits::ContextFactory,
};

use super::{ContextFactoryStandard, ContextStandard};
use crate::configuration::ConfigurationMap;

fn parent() -> (AppContext, ContextFactoryStandard) {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "parent".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    (ctx, ContextFactoryStandard::new())
}

#[tokio::test]
async fn generates_unique_trace_ids() {
    let (parent, factory) = parent();
    let a = factory.create(&parent, None).await.unwrap();
    let b = factory.create(&parent, None).await.unwrap();

    assert_ne!(a.trace_id(), "");
    assert_ne!(a.trace_id(), "parent");
    assert_ne!(a.trace_id(), b.trace_id());
}

#[tokio::test]
async fn uses_given_trace_id() {
    let (parent, factory) = parent();
    let ctx = factory
        .create(&parent, Some("cli".to_string()))
        .await
        .unwrap();

    assert_eq!(ctx.trace_id(), "cli");
    assert_eq!(
        serde_json::to_value(&ctx).unwrap(),
        serde_json::json!({ "trace_id": "cli" })
    );
}

#[test]
fn default_log_attributes_are_the_trace_id() {
    let (ctx, _) = parent();
    assert_eq!(ctx.log_attributes(), LogAttributes::new("parent"));
}
