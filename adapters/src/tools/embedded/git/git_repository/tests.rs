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

/// Three commits: `add a`, `add b`, `change a`.
fn history() -> (TempDir, GitRepository) {
    let (dir, git) = repository();
    fs::write(dir.path().join("a.txt"), "one\n").unwrap();
    git.commit("add a").unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/b.txt"), "b\n").unwrap();
    git.commit("add b\n\nWith a body.").unwrap();
    fs::write(dir.path().join("a.txt"), "two\n").unwrap();
    git.commit("change a").unwrap();
    (dir, git)
}

fn subjects(commits: &[String]) -> Vec<&str> {
    commits
        .iter()
        .map(|commit| {
            commit
                .lines()
                .next()
                .unwrap()
                .rsplit_once(" Beaver ")
                .unwrap()
                .1
        })
        .collect()
}

#[test]
fn log_lists_commits_newest_first() {
    let (_dir, git) = history();
    let lines = git.log("HEAD", None, 0, 20).unwrap();

    assert_eq!(subjects(&lines), ["change a", "add b", "add a"]);
    assert_eq!(lines[0].split(' ').next().unwrap().len(), 10);
}

#[test]
fn log_lists_the_changed_files_of_each_commit() {
    let (dir, git) = history();
    fs::rename(dir.path().join("a.txt"), dir.path().join("docs/a.txt")).unwrap();
    fs::remove_file(dir.path().join("docs/b.txt")).unwrap();
    fs::write(dir.path().join("c.txt"), "c\n").unwrap();
    git.commit("move a, drop b, add c").unwrap();

    let files = |commit: &String| commit.lines().skip(1).collect::<Vec<_>>().join("\n");
    let commits = git.log("HEAD", None, 0, 20).unwrap();
    assert_eq!(
        files(&commits[0]),
        "  A c.txt\n  R a.txt -> docs/a.txt\n  D docs/b.txt"
    );
    assert_eq!(files(&commits[1]), "  M a.txt");
    assert_eq!(files(&commits[2]), "  A docs/b.txt");
}

#[test]
fn log_shortens_long_file_lists() {
    let (dir, git) = repository();
    for i in 0..25 {
        fs::write(dir.path().join(format!("{:02}.txt", i)), "x").unwrap();
    }
    git.commit("many").unwrap();

    let commit = &git.log("HEAD", None, 0, 1).unwrap()[0];
    assert_eq!(commit.lines().count(), 1 + 20 + 1);
    assert!(commit.ends_with("  ... 5 more files"));
}

#[test]
fn log_pages_with_skip_and_limit() {
    let (_dir, git) = history();
    assert_eq!(subjects(&git.log("HEAD", None, 1, 1).unwrap()), ["add b"]);
    assert_eq!(
        subjects(&git.log("HEAD~1", None, 0, 20).unwrap()),
        ["add b", "add a"]
    );
}

#[test]
fn log_filters_by_file_or_directory() {
    let (_dir, git) = history();
    assert_eq!(
        subjects(&git.log("HEAD", Some("a.txt"), 0, 20).unwrap()),
        ["change a", "add a"]
    );
    assert_eq!(
        subjects(&git.log("HEAD", Some("docs"), 0, 20).unwrap()),
        ["add b"]
    );
    assert!(
        git.log("HEAD", Some("missing.txt"), 0, 20)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn show_prints_only_the_commits_changes() {
    let (_dir, git) = history();
    let shown = git.show("HEAD~1", None).unwrap();

    assert!(shown.starts_with("commit "));
    assert!(shown.contains("Author: Beaver <beaver@example.com>"));
    assert!(shown.contains("    add b\n    \n    With a body.\n"));
    assert!(shown.contains("docs/b.txt"));
    assert!(shown.contains("+b\n"));
    assert!(!shown.contains("a.txt"));
}

#[test]
fn show_diffs_against_the_parent_or_nothing_for_the_first_commit() {
    let (_dir, git) = history();
    let changed = git.show("HEAD", None).unwrap();
    assert!(changed.contains("-one\n"));
    assert!(changed.contains("+two\n"));

    let first = git.show("HEAD~2", None).unwrap();
    assert!(first.contains("+one\n"));
}

#[test]
fn show_filters_by_file() {
    let (_dir, git) = history();
    let shown = git.show("HEAD~1", Some("a.txt")).unwrap();
    assert!(shown.contains("    add b"));
    assert!(!shown.contains("docs/b.txt"));
}

#[test]
fn unknown_revisions_fail() {
    let (_dir, git) = history();
    assert!(git.log("missing", None, 0, 20).is_err());
    assert!(git.show("missing", None).is_err());
}
