// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::TempDir;

use super::{
    Disk,
    helpers::{absolute_target, relative_target},
};
use crate::file_system::tests::{context, path};
use app_domains::core::{
    models::{
        AppError,
        file::{self, Chroot},
    },
    traits::FileSystem,
};

const MODES: [file::WriteMode; 4] = [
    file::WriteMode::Truncate,
    file::WriteMode::Append,
    file::WriteMode::At(0),
    file::WriteMode::CreateNew,
];

/// A root next to a directory `outside` with `secret.txt` and a directory
/// `root-sibling` with `sibling.txt`. The root has `inside.txt` and symlinks
/// leading outside: `absolute`, `relative`, the chain `chain` → `chained` →
/// outside, the file link `secret-link`, the dangling `dangling` →
/// `outside/new.txt` and `sibling` → `root-sibling`.
struct Fixture {
    parent: TempDir,
    disk: Disk,
}

impl Fixture {
    fn new() -> Self {
        let parent = TempDir::new().unwrap();
        let root = parent.path().join("root");
        let outside = parent.path().join("outside");
        let sibling = parent.path().join("root-sibling");
        for dir in [&root, &outside, &sibling] {
            fs::create_dir(dir).unwrap();
        }
        fs::write(outside.join("secret.txt"), "secret").unwrap();
        fs::write(sibling.join("sibling.txt"), "sibling").unwrap();
        fs::write(root.join("inside.txt"), "inside").unwrap();
        symlink(&outside, root.join("absolute")).unwrap();
        symlink("../outside", root.join("relative")).unwrap();
        symlink(&outside, root.join("chained")).unwrap();
        symlink("chained", root.join("chain")).unwrap();
        symlink("../outside/secret.txt", root.join("secret-link")).unwrap();
        symlink("../outside/new.txt", root.join("dangling")).unwrap();
        symlink("../root-sibling", root.join("sibling")).unwrap();

        let disk = Disk::new(&context(), &root.to_string_lossy()).unwrap();
        Self { parent, disk }
    }

    fn root(&self) -> PathBuf {
        self.parent.path().join("root")
    }

    fn outside(&self) -> PathBuf {
        self.parent.path().join("outside")
    }

    /// Another `Disk` on the same root, for a `Chroot`.
    fn shared(&self) -> Arc<dyn FileSystem + Send + Sync> {
        Arc::new(Disk::new(&context(), &self.root().to_string_lossy()).unwrap())
    }

    fn assert_escape<T>(&self, operation: &str, result: Result<T, AppError>) {
        let error = self.assert_failed(operation, result);
        assert_eq!(
            error.translation_key, "access_violation",
            "{}: {}",
            operation, error.internal_message
        );
    }

    /// Fails without naming the host path.
    fn assert_failed<T>(&self, operation: &str, result: Result<T, AppError>) -> AppError {
        let Err(error) = result else {
            panic!("{operation} succeeded");
        };
        let host = self.parent.path().to_string_lossy().to_string();
        assert!(
            !error.internal_message.contains(&host),
            "{} leaks the host path: {}",
            operation,
            error.internal_message
        );
        error
    }

    fn assert_outside_unchanged(&self) {
        for (dir, name, content) in [
            ("outside", "secret.txt", "secret"),
            ("root-sibling", "sibling.txt", "sibling"),
        ] {
            let dir = self.parent.path().join(dir);
            assert_eq!(names(&dir), [name]);
            assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), content);
        }
        assert_eq!(
            names(self.parent.path()),
            ["outside", "root", "root-sibling"]
        );
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn escaping_symlinks_are_refused() {
    let fixture = Fixture::new();
    let (ctx, disk) = (context(), &fixture.disk);

    for link in ["absolute", "relative", "chain", "sibling"] {
        let at = |rest: &str| path(&format!("/{link}{rest}"));
        fixture.assert_escape("canonicalize", disk.canonicalize(&ctx, &at("")).await);
        fixture.assert_escape("metadata", disk.metadata(&ctx, &at("/secret.txt")).await);
        fixture.assert_escape("read", disk.read(&ctx, &at("/secret.txt"), None).await);
        fixture.assert_escape("read_dir", disk.read_dir(&ctx, &at("")).await);
        fixture.assert_escape(
            "write",
            disk.write(&ctx, &at("/new.txt"), b"x", file::WriteMode::Truncate)
                .await,
        );
        fixture.assert_escape(
            "write through link",
            disk.write(&ctx, &at("/secret.txt"), b"x", file::WriteMode::Truncate)
                .await,
        );
        fixture.assert_escape("mkdir", disk.mkdir(&ctx, &at("/dir"), false).await);
        fixture.assert_escape("mkdir -p", disk.mkdir(&ctx, &at("/a/b"), true).await);
        fixture.assert_escape("unlink", disk.unlink(&ctx, &at("/secret.txt"), false).await);
        fixture.assert_escape(
            "copy from",
            disk.copy(&ctx, &at("/secret.txt"), &path("/copy.txt"))
                .await,
        );
        fixture.assert_escape(
            "copy to",
            disk.copy(&ctx, &path("/inside.txt"), &at("/copy.txt"))
                .await,
        );
        fixture.assert_escape(
            "rename from",
            disk.rename(&ctx, &at("/secret.txt"), &path("/moved.txt"))
                .await,
        );
        fixture.assert_escape(
            "rename to",
            disk.rename(&ctx, &path("/inside.txt"), &at("/moved.txt"))
                .await,
        );
        fixture.assert_escape(
            "symlink",
            disk.symlink(&ctx, &path("/inside.txt"), &at("/link")).await,
        );
        fixture.assert_escape("materialize", disk.materialize(&ctx, &at("")).await);
    }
    for link in ["/absolute", "/relative", "/sibling"] {
        fixture.assert_escape(link, disk.read_link(&ctx, &path(link)).await);
    }

    fixture.assert_outside_unchanged();
    assert_eq!(
        fs::read_to_string(fixture.root().join("inside.txt")).unwrap(),
        "inside"
    );
}

#[tokio::test]
async fn escaping_leaf_links_are_not_followed() {
    let fixture = Fixture::new();
    let (ctx, disk) = (context(), &fixture.disk);
    let link = path("/secret-link");

    let metadata = disk.metadata(&ctx, &link).await.unwrap();
    assert_eq!(metadata.kind, file::Kind::Symlink);
    fixture.assert_escape("canonicalize", disk.canonicalize(&ctx, &link).await);
    fixture.assert_escape("read", disk.read(&ctx, &link, None).await);
    for mode in MODES {
        let written = disk.write(&ctx, &link, b"x", mode).await;
        fixture.assert_failed(&format!("write {mode:?}"), written);
    }
    fixture.assert_escape(
        "copy to",
        disk.copy(&ctx, &path("/inside.txt"), &link).await,
    );
    fixture.assert_escape(
        "copy from",
        disk.copy(&ctx, &link, &path("/copy.txt")).await,
    );

    let moved = path("/moved-link");
    disk.rename(&ctx, &link, &moved).await.unwrap();
    fixture.assert_escape("read moved", disk.read(&ctx, &moved, None).await);
    disk.unlink(&ctx, &moved, false).await.unwrap();
    assert!(!fixture.root().join("moved-link").exists());

    symlink("../outside/secret.txt", fixture.root().join("secret-link")).unwrap();
    disk.rename(&ctx, &path("/inside.txt"), &link)
        .await
        .unwrap();
    let replaced = fs::symlink_metadata(fixture.root().join("secret-link")).unwrap();
    assert!(replaced.is_file());
    assert_eq!(
        fs::read_to_string(fixture.root().join("secret-link")).unwrap(),
        "inside"
    );
    fixture.assert_outside_unchanged();
}

#[tokio::test]
async fn dangling_links_out_of_the_root_create_nothing() {
    let fixture = Fixture::new();
    let (ctx, disk) = (context(), &fixture.disk);
    let dangling = path("/dangling");

    for mode in MODES {
        let written = disk.write(&ctx, &dangling, b"x", mode).await;
        fixture.assert_failed(&format!("write {mode:?}"), written);
    }
    fixture.assert_failed(
        "copy to",
        disk.copy(&ctx, &path("/inside.txt"), &dangling).await,
    );
    fixture.assert_failed("mkdir", disk.mkdir(&ctx, &dangling, false).await);
    fixture.assert_failed("mkdir -p", disk.mkdir(&ctx, &dangling, true).await);
    fixture.assert_failed(
        "mkdir -p below",
        disk.mkdir(&ctx, &path("/dangling/x"), true).await,
    );
    fixture.assert_failed("canonicalize", disk.canonicalize(&ctx, &dangling).await);
    fixture.assert_failed("read", disk.read(&ctx, &dangling, None).await);
    fixture.assert_outside_unchanged();
}

#[tokio::test]
async fn listing_does_not_descend_into_escaping_links() {
    let fixture = Fixture::new();
    let (ctx, disk) = (context(), &fixture.disk);

    let listed = disk.read_dir(&ctx, &file::Path::root()).await.unwrap();
    for entry in &listed {
        let expected = match entry.path.as_str() {
            "/inside.txt" => file::Kind::File,
            _ => file::Kind::Symlink,
        };
        assert_eq!(entry.metadata.kind, expected, "{}", entry.path);
    }
    assert_eq!(listed.len(), 8);

    let all = 0..=usize::MAX;
    let found = disk
        .find(&ctx, &file::Path::root(), None, &[], all.clone(), &[], None)
        .await
        .unwrap();
    let found: Vec<&file::Path> = found.iter().map(|entry| &entry.path).collect();
    let mut expected = vec![file::Path::root()];
    expected.extend(listed.iter().map(|entry| entry.path.clone()));
    assert_eq!(found, expected.iter().collect::<Vec<_>>());

    for link in ["/absolute", "/sibling", "/dangling"] {
        let found = disk
            .find(&ctx, &path(link), None, &[], all.clone(), &[], None)
            .await
            .unwrap();
        let found: Vec<&str> = found.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(found, [link]);
    }
}

#[tokio::test]
async fn recursive_unlink_does_not_follow_escaping_links() {
    let fixture = Fixture::new();
    let ctx = context();
    let dir = fixture.root().join("dir/nested");
    fs::create_dir_all(&dir).unwrap();
    symlink(fixture.outside(), dir.join("absolute")).unwrap();
    symlink("../../../outside", dir.join("relative")).unwrap();
    symlink("../../../outside/secret.txt", dir.join("file")).unwrap();
    symlink("../../../root-sibling", dir.join("sibling")).unwrap();
    symlink("../..", dir.join("root")).unwrap();

    fixture
        .disk
        .unlink(&ctx, &path("/dir"), true)
        .await
        .unwrap();
    assert!(!fixture.root().join("dir").exists());
    assert!(fixture.root().join("inside.txt").exists());
    fixture.assert_outside_unchanged();
}

/// A link stored as `../t.txt` points elsewhere after `rename`. Moved out of
/// its directory, it points outside of the chroot `/work`.
#[tokio::test]
async fn moved_relative_links_stay_inside_the_chroot() {
    let fixture = Fixture::new();
    let ctx = context();
    let fs_port = fixture.shared();
    let whole = Chroot::new(&ctx, fs_port.clone(), "/", false)
        .await
        .unwrap();
    whole.mkdir(&ctx, "/work/a/b", true).await.unwrap();
    let work = Chroot::new(&ctx, fs_port, "/work", false).await.unwrap();
    work.write(&ctx, "/a/t.txt", b"inner", file::WriteMode::Truncate)
        .await
        .unwrap();
    work.symlink(&ctx, "/a/t.txt", "/a/b/l").await.unwrap();
    assert_eq!(work.read_to_string(&ctx, "/a/b/l").await.unwrap(), "inner");

    work.rename(&ctx, "/a/b/l", "/l").await.unwrap();
    let planted = fixture.root().join("t.txt");
    assert_eq!(
        fs::read_link(fixture.root().join("work/l")).unwrap(),
        Path::new("../t.txt")
    );
    fs::write(&planted, "planted").unwrap();

    fixture.assert_failed("read", work.read(&ctx, "/l", None).await);
    fixture.assert_failed("read_link", work.read_link(&ctx, "/l").await);
    fixture.assert_failed("copy from", work.copy(&ctx, "/l", "/c").await);
    fixture.assert_failed("copy to", work.copy(&ctx, "/a/t.txt", "/l").await);
    for mode in MODES {
        let written = work.write(&ctx, "/l", b"x", mode).await;
        fixture.assert_failed(&format!("write {mode:?}"), written);
    }
    assert_eq!(fs::read_to_string(&planted).unwrap(), "planted");
}

#[tokio::test]
async fn chroot_creation_does_not_follow_escaping_links() {
    let fixture = Fixture::new();
    let ctx = context();
    let before = names(&fixture.root());
    for chroot in [
        "/absolute/new",
        "/relative/new",
        "/chain/new/deeper",
        "/sibling/new",
        "/secret-link/new",
        "/dangling",
        "/dangling/new",
    ] {
        let created = Chroot::new(&ctx, fixture.shared(), chroot, true).await;
        fixture.assert_failed(chroot, created);
    }
    assert_eq!(names(&fixture.root()), before);
    fixture.assert_outside_unchanged();
}

#[tokio::test]
async fn escaping_symlinks_can_be_removed() {
    let fixture = Fixture::new();
    let ctx = context();
    fixture
        .disk
        .unlink(&ctx, &path("/absolute"), true)
        .await
        .unwrap();
    fixture
        .disk
        .unlink(&ctx, &path("/relative"), false)
        .await
        .unwrap();

    assert!(!fixture.root().join("absolute").exists());
    assert!(fixture.outside().join("secret.txt").exists());
}

#[tokio::test]
async fn symlinks_are_stored_relative() {
    let fixture = Fixture::new();
    let ctx = context();
    fixture.disk.mkdir(&ctx, &path("/a/b"), true).await.unwrap();
    fixture
        .disk
        .symlink(&ctx, &path("/inside.txt"), &path("/a/b/link"))
        .await
        .unwrap();

    assert_eq!(
        fs::read_link(fixture.root().join("a/b/link")).unwrap(),
        Path::new("../../inside.txt")
    );
    assert_eq!(
        fixture
            .disk
            .read_link(&ctx, &path("/a/b/link"))
            .await
            .unwrap(),
        path("/inside.txt")
    );
    fixture.assert_escape(
        "read_link outside",
        fixture.disk.read_link(&ctx, &path("/relative")).await,
    );
}

#[tokio::test]
async fn materializes_directories_inside_the_root() {
    let fixture = Fixture::new();
    let ctx = context();
    fixture
        .disk
        .mkdir(&ctx, &path("/repo"), false)
        .await
        .unwrap();
    fixture
        .disk
        .symlink(&ctx, &path("/repo"), &path("/repo-link"))
        .await
        .unwrap();

    let local = fixture
        .disk
        .materialize(&ctx, &path("/repo-link"))
        .await
        .unwrap();
    assert_eq!(
        local.path(),
        fixture.root().canonicalize().unwrap().join("repo")
    );
    local.sync(&ctx).await.unwrap();

    let file = fixture.disk.materialize(&ctx, &path("/inside.txt")).await;
    assert!(file.is_err());
    for link in ["/absolute", "/sibling"] {
        fixture.assert_escape(link, fixture.disk.materialize(&ctx, &path(link)).await);
    }
}

#[test]
fn a_missing_root_fails() {
    let parent = TempDir::new().unwrap();
    let missing = parent.path().join("missing");
    assert!(Disk::new(&context(), &missing.to_string_lossy()).is_err());
}

#[test]
fn link_targets_round_trip() {
    let ctx = context();
    for (target, link, content) in [
        ("/a/b/c", "/a/x/l", "../b/c"),
        ("/b", "/a/l", "../b"),
        ("/", "/a/b/l", "../.."),
        ("/a/t", "/a/l", "t"),
        ("/a", "/a/l", "."),
        ("/x/y", "/l", "x/y"),
        ("/a/l/x", "/a/l", "l/x"),
        ("/a-b/c", "/a/l", "../a-b/c"),
    ] {
        let (target, link) = (path(target), path(link));
        let relative = relative_target(&target, &link);
        assert_eq!(relative, Path::new(content), "{link} → {target}");
        assert_eq!(
            absolute_target(&ctx, &link, &relative).unwrap(),
            target,
            "{link} → {target}"
        );
    }
}

#[test]
fn absolute_target_rejects_escapes() {
    let ctx = context();
    for (link, content) in [
        ("/l", "/etc/passwd"),
        ("/a/l", "/a"),
        ("/l", ".."),
        ("/a/l", "../.."),
        ("/a/b/l", "../x/../../../y"),
        ("/a/l", "./../../a"),
    ] {
        let target = absolute_target(&ctx, &path(link), Path::new(content));
        let error = target.unwrap_err();
        assert_eq!(
            error.translation_key, "access_violation",
            "{link} {content}"
        );
    }
}
