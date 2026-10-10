// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::fs;

use git2::Repository as Git2Repository;
use tempfile::TempDir;

use super::GitRepository;
use crate::tools::embedded::git::Repository;

fn repository() -> (TempDir, GitRepository) {
    let dir = tempfile::tempdir().unwrap();
    Git2Repository::init(dir.path()).unwrap();
    let config = Repository {
        url: "https://example.com/repo.git".to_string(),
        email: "beaver@example.com".to_string(),
        display_name: Some("Beaver".to_string()),
        username: None,
        ssh_key: None,
        password: None,
        known_hosts: vec![],
    };
    let git = GitRepository::new(config, &dir.path().to_string_lossy());
    (dir, git)
}

fn commit_count(dir: &TempDir) -> usize {
    let repo = Git2Repository::open(dir.path()).unwrap();
    let Ok(head) = repo.head() else {
        return 0;
    };
    let mut walk = repo.revwalk().unwrap();
    walk.push(head.target().unwrap()).unwrap();
    walk.count()
}

fn head_has_file(dir: &TempDir, name: &str) -> bool {
    let repo = Git2Repository::open(dir.path()).unwrap();
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    tree.get_name(name).is_some()
}

#[test]
fn commits_new_and_modified_files() {
    let (dir, git) = repository();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    git.commit("add a").unwrap();

    fs::write(dir.path().join("a.txt"), "two").unwrap();
    git.commit("change a").unwrap();

    assert_eq!(commit_count(&dir), 2);
}

#[test]
fn refuses_to_create_an_empty_commit() {
    let (dir, git) = repository();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    git.commit("add a").unwrap();

    let err = git.commit("nothing changed").unwrap_err();

    assert!(err.message().contains("Nothing to commit"));
    assert_eq!(commit_count(&dir), 1);
}

#[test]
fn refuses_an_empty_first_commit() {
    let (dir, git) = repository();
    assert!(git.commit("empty").is_err());
    assert_eq!(commit_count(&dir), 0);
}

#[test]
fn commits_deleted_files() {
    let (dir, git) = repository();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    fs::write(dir.path().join("b.txt"), "two").unwrap();
    git.commit("add files").unwrap();

    fs::remove_file(dir.path().join("a.txt")).unwrap();
    git.commit("delete a").unwrap();

    assert_eq!(commit_count(&dir), 2);
    assert!(!head_has_file(&dir, "a.txt"));
    assert!(head_has_file(&dir, "b.txt"));
}

#[test]
fn ignored_files_are_not_committed() {
    let (dir, git) = repository();
    fs::write(dir.path().join(".gitignore"), "secret.txt\n").unwrap();
    git.commit("add gitignore").unwrap();

    fs::write(dir.path().join("secret.txt"), "token").unwrap();
    assert!(git.commit("only ignored changes").is_err());
    assert!(!head_has_file(&dir, "secret.txt"));
}

#[test]
fn commits_new_files_including_new_directories() {
    let (dir, git) = repository();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    git.commit("add a").unwrap();

    fs::write(dir.path().join("b.txt"), "two").unwrap();
    fs::create_dir_all(dir.path().join("src/nested")).unwrap();
    fs::write(dir.path().join("src/nested/c.rs"), "fn main() {}").unwrap();
    git.commit("add b and c").unwrap();

    let repo = Git2Repository::open(dir.path()).unwrap();
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    assert!(tree.get_name("b.txt").is_some());
    assert!(
        tree.get_path(std::path::Path::new("src/nested/c.rs"))
            .is_ok()
    );
    assert_eq!(commit_count(&dir), 2);
}
