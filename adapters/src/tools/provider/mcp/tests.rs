// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::core::models::{AppContext, ErrorKind};
use rmcp::{ClientLifecycleMode, model::ProtocolVersion};

use super::{McpServer, McpSession};
use crate::{configuration::ConfigurationMap, logger::LoggerRecording};

fn context() -> AppContext {
    AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    )
}

fn session(protocol_version: Option<&str>) -> McpSession {
    McpSession::new(McpServer {
        name: "test".to_string(),
        description: String::new(),
        url: "http://localhost/mcp".to_string(),
        credentials: None,
        protocol_version: protocol_version.map(str::to_string),
    })
}

#[test]
fn without_pin_probes_discover_and_falls_back_to_initialize() {
    let (mode, version) = session(None).lifecycle(&context()).unwrap();
    assert_eq!(
        mode,
        ClientLifecycleMode::Auto {
            preferred_versions: vec![ProtocolVersion::LATEST],
            legacy_version: Some(ProtocolVersion::LATEST_WITH_INITIALIZE),
        }
    );
    assert_eq!(version, ProtocolVersion::LATEST_WITH_INITIALIZE);
}

#[test]
fn pinned_legacy_version_uses_initialize() {
    let (mode, version) = session(Some("2025-03-26")).lifecycle(&context()).unwrap();
    assert_eq!(mode, ClientLifecycleMode::Initialize);
    assert_eq!(version, ProtocolVersion::V_2025_03_26);
}

#[test]
fn pinned_version_without_handshake_uses_discover() {
    let (mode, version) = session(Some("2026-07-28")).lifecycle(&context()).unwrap();
    assert_eq!(
        mode,
        ClientLifecycleMode::Discover {
            preferred_versions: vec![ProtocolVersion::V_2026_07_28],
        }
    );
    assert_eq!(version, ProtocolVersion::V_2026_07_28);
}

#[test]
fn unknown_pinned_version_is_a_validation_error() {
    let error = session(Some("1999-01-01"))
        .lifecycle(&context())
        .unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Validation));
}

#[test]
fn server_deserializes_with_and_without_protocol_version() {
    let server: McpServer = serde_json::from_value(serde_json::json!({
        "name": "a",
        "url": "http://localhost/mcp",
    }))
    .unwrap();
    assert_eq!(server.protocol_version, None);

    let server: McpServer = serde_json::from_value(serde_json::json!({
        "name": "a",
        "url": "http://localhost/mcp",
        "protocol_version": "2025-06-18",
    }))
    .unwrap();
    assert_eq!(server.protocol_version.as_deref(), Some("2025-06-18"));
}
