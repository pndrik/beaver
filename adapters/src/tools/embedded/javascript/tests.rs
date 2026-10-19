// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, fs, sync::Arc};

use serde_json::json;
use tempfile::TempDir;

use super::tools::Run;
use crate::{configuration::ConfigurationMap, logger::LoggerRecording, tools::sandbox::Boa};
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::{
        Agent, AgentCacheLevel, AgentInference, Compression, Conversation, Message, Model, Options,
    },
    tools::{
        models::{Call, CallValue, ToolPermission},
        traits::Tool,
    },
};

fn context(configuration: ConfigurationMap) -> AppContext {
    AppContext::new(
        "test".to_string(),
        Arc::new(configuration),
        Arc::new(LoggerRecording::new()),
    )
}

fn conversation(ctx: &AppContext, chroot: &TempDir) -> Conversation {
    let agent = Agent::new(
        "agent".to_string(),
        "agent".to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    Conversation::new(
        ctx,
        Message::system(String::new(), &agent),
        agent,
        vec![],
        &chroot.path().to_string_lossy(),
    )
    .unwrap()
}

fn permission(scopes: &[&str]) -> ToolPermission {
    ToolPermission {
        name: "javascript_run".to_string(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
    }
}

fn arguments(pairs: &[(&str, &str)]) -> Call {
    Call::new(
        "javascript_run".to_string(),
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), CallValue::String(value.to_string())))
            .collect::<HashMap<_, _>>(),
    )
}

/// A chroot with `data.json` and a symlink `escape` pointing outside of it.
fn chroot() -> (TempDir, TempDir) {
    let chroot = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(
        chroot.path().join("data.json"),
        r#"[{"n":1},{"n":2},{"n":3}]"#,
    )
    .unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), chroot.path().join("escape")).unwrap();
    (chroot, outside)
}

async fn run_with(
    configuration: ConfigurationMap,
    chroot: &TempDir,
    scopes: &[&str],
    call: Call,
) -> Result<String, AppError> {
    let ctx = context(configuration);
    let mut conversation = conversation(&ctx, chroot);
    Run::new(Arc::new(Boa::new()))
        .call(&ctx, &mut conversation, permission(scopes), &call)
        .await
}

async fn code(chroot: &TempDir, code: &str) -> Result<String, AppError> {
    run_with(
        ConfigurationMap::default(),
        chroot,
        &[],
        arguments(&[("code", code)]),
    )
    .await
}

#[tokio::test]
async fn returns_console_output_and_value() {
    let (chroot, _outside) = chroot();
    let result = code(
        &chroot,
        "console.log('a'); console.error('b'); return { x: 1 };",
    )
    .await
    .unwrap();
    assert_eq!(result, "a\nb\n{\"x\":1}");

    assert_eq!(code(&chroot, "return 'plain';").await.unwrap(), "plain");
    assert_eq!(code(&chroot, "console.log('only');").await.unwrap(), "only");
    assert_eq!(code(&chroot, "").await.unwrap(), "");
}

#[tokio::test]
async fn errors_carry_the_console_output() {
    let (chroot, _outside) = chroot();
    let err = code(&chroot, "console.log('before'); throw new Error('boom');")
        .await
        .unwrap_err();
    assert!(
        err.internal_message.contains("boom"),
        "{}",
        err.internal_message
    );
    assert!(err.internal_message.contains("Console output:\nbefore"));
}

#[tokio::test]
async fn reads_lists_writes_and_appends_files() {
    let (chroot, _outside) = chroot();
    fs::create_dir(chroot.path().join("out")).unwrap();
    let result = code(
        &chroot,
        r#"
        const data = JSON.parse(fs.readFile("/data.json"));
        fs.writeFile("/out/sum.txt", String(data.reduce((a, b) => a + b.n, 0)));
        fs.appendFile("/out/sum.txt", "!");
        fs.appendFile("/out/new.txt", "x");
        return fs.readDir("/");
        "#,
    )
    .await
    .unwrap();

    assert_eq!(result, r#"["data.json","escape/","out/"]"#);
    assert_eq!(
        fs::read_to_string(chroot.path().join("out/sum.txt")).unwrap(),
        "6!"
    );
    assert_eq!(
        fs::read_to_string(chroot.path().join("out/new.txt")).unwrap(),
        "x"
    );
}

#[tokio::test]
async fn blocks_escaping_paths() {
    let (chroot, outside) = chroot();
    for script in [
        r#"return fs.readFile("/../secret.txt");"#,
        r#"return fs.readFile("/escape/secret.txt");"#,
        r#"return fs.readDir("/escape");"#,
        r#"fs.writeFile("/escape/secret.txt", "x");"#,
        r#"fs.writeFile("/../x.txt", "x");"#,
        r#"fs.appendFile("/missing/x.txt", "x");"#,
        r#"return fs.readFile("data.json");"#,
    ] {
        let err = code(&chroot, script).await.unwrap_err();
        assert!(err.internal_message.contains("Error"), "{}", script);
    }
    assert_eq!(
        fs::read_to_string(outside.path().join("secret.txt")).unwrap(),
        "secret"
    );

    let caught = code(
        &chroot,
        r#"try { fs.readFile("/escape/secret.txt"); } catch (e) { return "caught"; }"#,
    )
    .await
    .unwrap();
    assert_eq!(caught, "caught");
}

#[tokio::test]
async fn runs_modules_by_path() {
    let (chroot, _outside) = chroot();
    fs::create_dir(chroot.path().join("lib")).unwrap();
    fs::write(
        chroot.path().join("lib/sum.js"),
        "export const sum = (list) => list.reduce((a, b) => a + b, 0);",
    )
    .unwrap();
    fs::write(
        chroot.path().join("lib/function.js"),
        "import { sum } from '/lib/sum.js';\nexport default async function () { console.log('hi'); return { total: sum([3, 4]) }; }",
    )
    .unwrap();

    let path = |path| arguments(&[("path", path)]);
    let run = |call| run_with(ConfigurationMap::default(), &chroot, &[], call);
    assert_eq!(
        run(path("/lib/function.js")).await.unwrap(),
        "hi\n{\"total\":7}"
    );
    assert!(run(path("/missing.js")).await.is_err());
    assert!(run(path("lib/function.js")).await.is_err());
}

#[tokio::test]
async fn requires_exactly_one_of_code_or_path() {
    let (chroot, _outside) = chroot();
    let run = |call| run_with(ConfigurationMap::default(), &chroot, &[], call);
    assert!(
        run(arguments(&[("code", "return 1;"), ("path", "/a.js")]))
            .await
            .is_err()
    );
    assert!(run(arguments(&[])).await.is_err());
}

#[tokio::test]
async fn imports_resolve_inside_the_chroot() {
    let (chroot, _outside) = chroot();
    fs::write(chroot.path().join("lib.js"), "export const two = 2;").unwrap();

    assert_eq!(
        code(
            &chroot,
            r#"const { two } = await import("./lib.js"); return two;"#
        )
        .await
        .unwrap(),
        "2"
    );
    for script in [
        r#"await import("lodash");"#,
        r#"await import("../outside.js");"#,
        r#"await import("/escape/secret.txt");"#,
    ] {
        assert!(code(&chroot, script).await.is_err(), "{}", script);
    }
}

#[tokio::test]
async fn fetch_needs_a_web_scope() {
    let (chroot, _outside) = chroot();
    let probe = |scopes: &'static [&'static str]| {
        run_with(
            ConfigurationMap::default(),
            &chroot,
            scopes,
            arguments(&[("code", "return typeof fetch;")]),
        )
    };

    assert_eq!(probe(&[]).await.unwrap(), "undefined");
    assert_eq!(probe(&["https://example.com"]).await.unwrap(), "undefined");
    assert_eq!(
        probe(&["web:^https://allowed\\.example/"]).await.unwrap(),
        "function"
    );
}

#[tokio::test]
async fn fetch_rejects_urls_that_match_no_web_scope() {
    let (chroot, _outside) = chroot();
    let err = run_with(
        ConfigurationMap::default(),
        &chroot,
        &["web:^https://allowed\\.example/"],
        arguments(&[("code", r#"await fetch("https://other.example/");"#)]),
    )
    .await
    .unwrap_err();
    assert!(
        err.internal_message.contains("URL not allowed"),
        "{}",
        err.internal_message
    );
}

#[tokio::test]
async fn invalid_web_scope_fails_before_running() {
    let (chroot, _outside) = chroot();
    let err = run_with(
        ConfigurationMap::default(),
        &chroot,
        &["web:("],
        arguments(&[("code", r#"fs.writeFile("/ran.txt", "x");"#)]),
    )
    .await
    .unwrap_err();
    assert!(err.internal_message.contains("Invalid web scope"));
    assert!(!chroot.path().join("ran.txt").exists());
}

#[tokio::test]
async fn configured_limits_stop_runaway_loops() {
    let (chroot, _outside) = chroot();
    let configuration = ConfigurationMap::from_iter([(
        "tools.configuration.javascript.loop_iteration_limit",
        json!(1000),
    )]);
    let err = run_with(
        configuration,
        &chroot,
        &[],
        arguments(&[("code", "while (true) {}")]),
    )
    .await
    .unwrap_err();
    assert!(
        err.internal_message.to_lowercase().contains("limit"),
        "{}",
        err.internal_message
    );
}
