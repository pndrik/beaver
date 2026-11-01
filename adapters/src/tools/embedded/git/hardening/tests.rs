// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use git2::{ConfigLevel, Repository};
use std::fs;
use tempfile::TempDir;

use super::{isolate, open_contained, validate_url};

/// A parent directory with the repository `repo` in it.
fn repository() -> (TempDir, std::path::PathBuf) {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("repo");
    Repository::init(&path).unwrap();
    (parent, path)
}

fn set(path: &std::path::Path, name: &str, value: &str) {
    Repository::open(path)
        .unwrap()
        .config()
        .unwrap()
        .open_level(ConfigLevel::Local)
        .unwrap()
        .set_str(name, value)
        .unwrap();
}

fn refused(path: &std::path::Path) -> String {
    match open_contained(path) {
        Ok(_) => panic!("opened '{}'", path.display()),
        Err(e) => e.message().to_string(),
    }
}

#[test]
fn opens_contained_repositories_without_symlinks() {
    let (_parent, path) = repository();
    let repository = open_contained(&path).unwrap();
    let config = repository
        .config()
        .unwrap()
        .open_level(ConfigLevel::Local)
        .unwrap();
    assert!(!config.get_bool("core.symlinks").unwrap());
}

#[test]
fn does_not_search_parent_directories() {
    let (_parent, path) = repository();
    fs::create_dir(path.join("sub")).unwrap();
    assert!(open_contained(&path.join("sub")).is_err());
}

#[test]
fn refuses_git_directories_outside() {
    let (parent, outside) = repository();
    let path = parent.path().join("pointer");
    fs::create_dir(&path).unwrap();
    fs::write(
        path.join(".git"),
        format!("gitdir: {}\n", outside.join(".git").display()),
    )
    .unwrap();
    assert!(refused(&path).contains("git directory is outside"));

    let linked = parent.path().join("linked");
    fs::create_dir(&linked).unwrap();
    std::os::unix::fs::symlink(outside.join(".git"), linked.join(".git")).unwrap();
    assert!(refused(&linked).contains("git directory is outside"));
}

#[test]
fn refuses_worktrees_and_includes() {
    for (name, value) in [
        ("core.worktree", ".."),
        ("include.path", "/etc/gitconfig"),
        ("includeIf.gitdir:/.path", "/etc/gitconfig"),
    ] {
        let (_parent, path) = repository();
        set(&path, name, value);
        let message = refused(&path);
        assert!(message.contains("not supported"), "{name}: {message}");
    }
}

#[test]
fn refuses_alternates() {
    let (parent, path) = repository();
    fs::write(
        path.join(".git/objects/info/alternates"),
        parent.path().join("objects").to_string_lossy().as_bytes(),
    )
    .unwrap();
    assert!(refused(&path).contains("alternates"));
}

#[test]
fn ignores_global_configuration() {
    isolate();
    for level in [ConfigLevel::System, ConfigLevel::Global, ConfigLevel::XDG] {
        // SAFETY: `isolate` already ran, nothing changes the search paths anymore.
        let path = unsafe { git2::opts::get_search_path(level) }.unwrap();
        assert!(path.as_bytes().is_empty(), "{level:?}");
    }
}

#[test]
fn allows_only_remote_urls() {
    for url in [
        "https://example.com/repo.git",
        "ssh://git@example.com/repo.git",
        "git@example.com:team/repo.git",
        "example.com:/srv/repo.git",
    ] {
        assert!(validate_url(url).is_ok(), "{}", url);
    }
    for url in [
        "file:///etc",
        "/srv/repo.git",
        "./repo",
        "../repo",
        "repo",
        "C:/repo",
        "http://example.com/repo.git",
        "ext::sh -c touch% /tmp/pwned",
        "-oProxyCommand=x:repo",
        "git://example.com/repo.git",
    ] {
        assert!(validate_url(url).is_err(), "{}", url);
    }
}
