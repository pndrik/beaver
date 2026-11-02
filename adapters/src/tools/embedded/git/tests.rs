// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use git2::Repository as Git2Repository;
use serde_json::json;
use std::{collections::HashMap, fs, os::unix::fs::symlink, sync::Arc};
use tempfile::TempDir;

use super::{configuration::get_repository_by_url, tools};
use crate::{
    configuration::ConfigurationMap,
    context::ContextStandard,
    file_system::{Disk, Memory},
};
use app_domains::{
    core::models::{AppContext, AppError, ErrorKind, file::Chroot},
    inference::models::{
        Agent, AgentInference, CacheLevel, Compression, Conversation, Message, Model, Options,
    },
    tools::{
        models::{Call, CallValue, ToolPermission},
        traits::Tool,
    },
};

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

fn conversation(chroot: Chroot) -> Conversation {
    let agent = Agent::new(
        "agent".to_string(),
        "agent".to_string(),
        String::new(),
        "You help.".to_string(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: CacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    Conversation::new(
        Message::system(String::new(), &agent),
        agent,
        vec![],
        chroot,
    )
}

async fn call(
    chroot: &Chroot,
    tool: &(dyn Tool + Send + Sync),
    scopes: &[&str],
    arguments: &[(&str, &str)],
) -> Result<String, AppError> {
    let ctx = context();
    let name = tool.name(&ctx).await.unwrap();
    let call = Call::new(
        name.clone(),
        arguments
            .iter()
            .map(|(key, value)| (key.to_string(), CallValue::String(value.to_string())))
            .collect::<HashMap<_, _>>(),
    );
    let permissions = ToolPermission {
        name,
        scopes: scopes
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
    };
    tool.call(&ctx, &mut conversation(chroot.clone()), permissions, &call)
        .await
}

/// A root with the repository `repo` and the symlink `escape` to a repository
/// outside of it.
async fn disk() -> (TempDir, Chroot) {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    fs::create_dir(&root).unwrap();
    Git2Repository::init(root.join("repo")).unwrap();
    Git2Repository::init(parent.path().join("outside")).unwrap();
    symlink(parent.path().join("outside"), root.join("escape")).unwrap();

    let ctx = context();
    let chroot = Chroot::new(
        &ctx,
        Arc::new(Disk::new(&ctx, &root.to_string_lossy()).unwrap()),
        "/",
        false,
    )
    .await
    .unwrap();
    (parent, chroot)
}

#[tokio::test]
async fn repositories_must_be_inside_the_chroot_and_scopes() {
    let (_parent, chroot) = disk().await;
    let log = |path: &'static str, scopes: &'static [&'static str]| {
        let chroot = chroot.clone();
        async move { call(&chroot, &tools::Log, scopes, &[("path", path)]).await }
    };

    let empty = log("/repo", &[]).await.unwrap_err();
    assert!(
        !empty.internal_message.contains("outside"),
        "{}",
        empty.internal_message
    );
    let outside = log("/escape", &[]).await.unwrap_err();
    assert_eq!(outside.translation_key, "file_not_found");
    assert!(log("/../outside", &[]).await.is_err());
    let unscoped = log("/repo", &["/other"]).await.unwrap_err();
    assert!(
        unscoped
            .internal_message
            .contains("not within allowed scopes")
    );
}

#[tokio::test]
async fn clones_only_from_remote_urls_into_the_chroot() {
    let (parent, chroot) = disk().await;
    let clone = |path: &'static str, url: &'static str| {
        let chroot = chroot.clone();
        async move { call(&chroot, &tools::Clone, &[], &[("path", path), ("url", url)]).await }
    };

    for url in ["file:///etc", "/srv/repo.git", "../outside"] {
        let error = clone("/new", url).await.unwrap_err();
        assert!(error.internal_message.contains("not allowed"), "{}", url);
    }
    for path in ["/escape/new", "/../new", "/repo", "/"] {
        assert!(
            clone(path, "https://example.com/repo.git").await.is_err(),
            "{}",
            path
        );
    }
    assert!(!parent.path().join("outside/new").exists());
    assert!(!parent.path().join("new").exists());
}

#[tokio::test]
async fn memory_file_systems_are_unsupported() {
    let ctx = context();
    let chroot = Chroot::new(&ctx, Arc::new(Memory::new()), "/", false)
        .await
        .unwrap();
    chroot.mkdir(&ctx, "/repo", false).await.unwrap();

    let error = call(&chroot, &tools::Log, &[], &[("path", "/repo")])
        .await
        .unwrap_err();
    assert_eq!(error.translation_key, "file_system_unsupported");
}

fn repositories(urls: &[&str]) -> AppContext {
    let repositories: Vec<_> = urls
        .iter()
        .map(|url| json!({ "url": url, "email": "bot@example.org" }))
        .collect();
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::from_iter([(
            "tools.configuration.git.repositories",
            json!(repositories),
        )])),
    )))
}

async fn lookup(urls: &[&str], url: &str) -> Result<String, AppError> {
    get_repository_by_url(&repositories(urls), url)
        .await
        .map(|repository| repository.email)
}

#[tokio::test]
async fn repository_regexes_match_whole_urls() {
    let team = ["^ssh://git@example\\.org/team/.*$"];
    assert!(
        lookup(&team, "ssh://git@example.org/team/repo.git")
            .await
            .is_ok()
    );
    for url in [
        "ssh://git@example.org/other/team/repo.git",
        "ssh://git@example.org.evil/team/repo.git",
        "https://evil.example/ssh://git@example.org/team/repo.git",
    ] {
        let err = lookup(&team, url).await.unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotFound, "{url}");
    }

    let either = ["^https://a\\.example/r\\.git|https://b\\.example/r\\.git$"];
    assert!(lookup(&either, "https://a.example/r.git").await.is_ok());
    assert!(lookup(&either, "https://b.example/r.git").await.is_ok());
    assert!(
        lookup(&either, "https://a.example/r.git.evil")
            .await
            .is_err()
    );
    assert!(lookup(&either, "https://x.b.example/r.git").await.is_err());
}

#[tokio::test]
async fn repository_literals_match_exactly() {
    let exact = ["https://example.org/team/repo.git"];
    assert!(
        lookup(&exact, "https://example.org/team/repo.git")
            .await
            .is_ok()
    );
    assert!(
        lookup(&exact, "https://example.org/team/repo.git/x")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn broken_repository_patterns_fail_every_lookup() {
    for (broken, message) in [
        ("^ssh://x", "Repository URL '^ssh://x' must end with '$'"),
        ("^($", "Repository URL '^($' is not a valid regex"),
    ] {
        let urls = ["https://example.org/repo.git", broken];
        let err = lookup(&urls, "https://example.org/repo.git")
            .await
            .unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
        assert_eq!(err.translation_key, "configuration_load_failed");
        assert!(
            err.internal_message.contains(message),
            "{}",
            err.internal_message
        );
    }
}
