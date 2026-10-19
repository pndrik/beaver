// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;

use app_domains::{
    core::models::LiteralOrRegex,
    tools::models::{
        SandboxFilesystemOperation::{Read, Write},
        SandboxScopes,
    },
};

fn scopes(value: serde_json::Value) -> SandboxScopes {
    serde_json::from_value(value).unwrap()
}

#[test]
fn values_are_regexes_only_with_a_caret() {
    assert!(matches!(
        LiteralOrRegex::parse("^https://a\\.example/").unwrap(),
        LiteralOrRegex::Regex(_)
    ));
    assert!(matches!(
        LiteralOrRegex::parse("https://a.example/").unwrap(),
        LiteralOrRegex::Literal(_)
    ));
    assert!(LiteralOrRegex::parse("^(").is_err());
    assert!(
        serde_json::from_value::<SandboxScopes>(json!({ "fetch": [{ "url": "^(" }] })).is_err()
    );
}

#[test]
fn nothing_is_allowed_by_default() {
    let none = scopes(json!({}));
    assert!(!none.allows_url("https://example.com/"));
    assert!(!none.allows_file("/a.txt", Read));
}

#[test]
fn fetch_scopes_allow_their_urls_and_methods() {
    let scopes = scopes(json!({ "fetch": [
        { "url": "https://exact.example/", "methods": ["GET"] },
        { "url": "^https://api\\.example/", "methods": ["get", "POST"] },
        { "url": "^https://any\\.example/" }
    ] }));

    assert!(scopes.allows_fetch("https://exact.example/", "GET"));
    assert!(!scopes.allows_fetch("https://exact.example/", "POST"));
    assert!(!scopes.allows_url("https://exact.example/more"));
    assert!(scopes.allows_fetch("https://api.example/v1", "GET"));
    assert!(scopes.allows_fetch("https://api.example/v1", "post"));
    assert!(!scopes.allows_fetch("https://api.example/v1", "DELETE"));
    assert!(scopes.allows_fetch("https://any.example/x", "DELETE"));
    assert!(!scopes.allows_url("https://other.example/"));
}

#[test]
fn filesystem_scopes_allow_their_paths_and_operations() {
    let scopes = scopes(json!({ "filesystem": [
        { "path": "/data", "operations": ["read"] },
        { "path": "/out/", "operations": ["read", "write"] },
        { "path": "^/tmp/[^/]+\\.json$", "operations": ["write"] }
    ] }));

    assert!(scopes.allows_file("/data", Read));
    assert!(scopes.allows_file("/data/a/b.json", Read));
    assert!(!scopes.allows_file("/database", Read));
    assert!(!scopes.allows_file("/data/a.json", Write));
    assert!(scopes.allows_file("/out/a.json", Write));
    assert!(scopes.allows_file("/tmp/a.json", Write));
    assert!(!scopes.allows_file("/tmp/a/b.json", Write));
    assert!(!scopes.allows_file("/secret.txt", Read));
}

#[test]
fn a_root_literal_covers_the_whole_chroot() {
    let scopes = scopes(json!({ "filesystem": [{ "path": "/", "operations": ["read"] }] }));
    assert!(scopes.allows_file("/", Read));
    assert!(scopes.allows_file("/a/b.txt", Read));
}
