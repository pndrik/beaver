// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use std::{ops::RangeInclusive, sync::Arc};

use app_adapters::file_system::Memory;
use app_domains::core::{
    models::{
        AppContext, AppError, ErrorKind,
        file::{self, Chroot},
    },
    traits::FileSystem,
};

use crate::context;

/// The whole file system with `/work/file.txt`, `/work/dir`, `/secret.txt` and
/// `/other`, and the chroot `/work` with links: `out` → `/other`,
/// `secret.txt` → `/secret.txt`, `inner` → `/work/file.txt`, `dir-link` →
/// `/work/dir`.
async fn fixture() -> (AppContext, Chroot, Chroot) {
    let ctx = context();
    let memory = Arc::new(Memory::new());
    let root = Chroot::new(&ctx, memory.clone(), "/", false).await.unwrap();
    root.mkdir(&ctx, "/work/dir", true).await.unwrap();
    root.mkdir(&ctx, "/other", false).await.unwrap();
    for (path, content) in [("/work/file.txt", "file"), ("/secret.txt", "secret")] {
        root.write(&ctx, path, content.as_bytes(), file::WriteMode::Truncate)
            .await
            .unwrap();
    }
    for (target, link) in [
        ("/other", "/work/out"),
        ("/secret.txt", "/work/secret.txt"),
        ("/work/file.txt", "/work/inner"),
        ("/work/dir", "/work/dir-link"),
    ] {
        root.symlink(&ctx, target, link).await.unwrap();
    }

    let work = Chroot::new(&ctx, memory, "/work", false).await.unwrap();
    (ctx, root, work)
}

fn assert_violation<T>(what: &str, result: Result<T, AppError>) {
    let Err(error) = result else {
        panic!("{what} succeeded");
    };
    assert_eq!(
        error.translation_key, "access_violation",
        "{}: {}",
        what, error.internal_message
    );
}

/// Refused for leading outside: `access_violation`, or the `file_not_found`
/// that missing paths give too.
fn assert_refused<T>(what: &str, result: Result<T, AppError>) {
    let Err(error) = result else {
        panic!("{what} succeeded");
    };
    assert!(
        ["access_violation", "file_not_found"].contains(&error.translation_key),
        "{}: {}",
        what,
        error.internal_message
    );
}

async fn text(ctx: &AppContext, chroot: &Chroot, path: &str) -> String {
    chroot.read_to_string(ctx, path).await.unwrap()
}

async fn chroot(
    ctx: &AppContext,
    memory: &Arc<Memory>,
    path: &str,
    create: bool,
) -> Result<Chroot, AppError> {
    Chroot::new(ctx, memory.clone(), path, create).await
}

/// A memory file system with the directory `/work/dir`, the file
/// `/work/file.txt` and the symlink `/work/dir-link` to `/work/dir`.
async fn memory(ctx: &AppContext) -> Arc<Memory> {
    let memory = Arc::new(Memory::new());
    let root = chroot(ctx, &memory, "/", false).await.unwrap();
    root.mkdir(ctx, "/work/dir", true).await.unwrap();
    root.write(ctx, "/work/file.txt", b"file", file::WriteMode::Truncate)
        .await
        .unwrap();
    root.symlink(ctx, "/work/dir", "/work/dir-link")
        .await
        .unwrap();
    memory
}

#[tokio::test]
async fn new_resolves_the_root_through_symlinks() {
    let ctx = context();
    let memory = memory(&ctx).await;
    let root = async |path| chroot(&ctx, &memory, path, false).await;

    assert!(root("/").await.unwrap().root().is_root());
    assert_eq!(
        root("/work/dir").await.unwrap().root().as_str(),
        "/work/dir"
    );
    assert_eq!(
        root("work/dir").await.unwrap_err().translation_key,
        "access_violation"
    );
    assert_eq!(
        root("/work/dir-link").await.unwrap().root().as_str(),
        "/work/dir"
    );
    assert_violation("parent", root("/work/..").await);
    assert_violation("file", root("/work/file.txt").await);
    assert_eq!(
        root("/missing").await.unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn new_creates_missing_directories() {
    let ctx = context();
    let memory = memory(&ctx).await;
    let created = chroot(&ctx, &memory, "/runs/trace", true).await.unwrap();
    assert_eq!(created.root().as_str(), "/runs/trace");
    let again = chroot(&ctx, &memory, "/runs/trace", true).await.unwrap();
    assert_eq!(again.root(), created.root());
    let through_link = chroot(&ctx, &memory, "/work/dir-link/nested", true)
        .await
        .unwrap();
    assert_eq!(through_link.root().as_str(), "/work/dir/nested");

    assert!(
        chroot(&ctx, &memory, "/work/file.txt/x", true)
            .await
            .is_err()
    );
    assert!(chroot(&ctx, &memory, "/missing", false).await.is_err());
}

#[tokio::test]
async fn symlinks_can_not_leave_a_sub_chroot() {
    let (ctx, root, work) = fixture().await;
    root.write(&ctx, "/other/kept.txt", b"kept", file::WriteMode::Truncate)
        .await
        .unwrap();
    assert_refused("read", work.read(&ctx, "/secret.txt", None).await);
    assert_refused(
        "read_to_string",
        work.read_to_string(&ctx, "/secret.txt").await,
    );
    assert_refused("metadata", work.metadata(&ctx, "/out/kept.txt").await);
    assert_refused("unlink", work.unlink(&ctx, "/out/kept.txt", false).await);
    assert_refused(
        "rename from",
        work.rename(&ctx, "/out/kept.txt", "/moved").await,
    );
    assert_refused("materialize", work.materialize(&ctx, "/out").await);
    assert_refused(
        "resolve missing leaf",
        work.resolve(&ctx, "/out/missing").await,
    );
    assert_refused("read_dir", work.read_dir(&ctx, "/out").await);
    assert_eq!(work.resolve(&ctx, "/out").await.unwrap().as_str(), "/out");
    assert_refused(
        "write",
        work.write(&ctx, "/out/new.txt", b"x", file::WriteMode::Truncate)
            .await,
    );
    assert_refused(
        "write through link",
        work.write(&ctx, "/secret.txt", b"x", file::WriteMode::Truncate)
            .await,
    );
    assert_refused("mkdir", work.mkdir(&ctx, "/out/new", false).await);
    assert_refused("mkdir -p", work.mkdir(&ctx, "/out/a/b", true).await);
    assert_refused(
        "copy from",
        work.copy(&ctx, "/secret.txt", "/copy.txt").await,
    );
    assert_refused(
        "copy to",
        work.copy(&ctx, "/file.txt", "/out/copy.txt").await,
    );
    assert_refused(
        "rename to",
        work.rename(&ctx, "/file.txt", "/out/moved").await,
    );
    assert_refused("symlink target", work.symlink(&ctx, "/out", "/again").await);
    assert_refused(
        "symlink in",
        work.symlink(&ctx, "/file.txt", "/out/l").await,
    );
    assert_refused(
        "find",
        work.find(&ctx, "/out", None, &[], ALL, &[], None).await,
    );
    assert_refused("read_link", work.read_link(&ctx, "/out").await);

    assert_eq!(text(&ctx, &root, "/secret.txt").await, "secret");
    assert_eq!(text(&ctx, &root, "/other/kept.txt").await, "kept");
    assert_eq!(root.read_dir(&ctx, "/other").await.unwrap().len(), 1);
}

#[tokio::test]
async fn unlink_removes_links_and_never_the_chroot() {
    let (ctx, root, work) = fixture().await;
    for chroot in [&root, &work] {
        for path in ["/", "//", "/."] {
            assert_violation("unlink root", chroot.unlink(&ctx, path, true).await);
        }
    }

    work.unlink(&ctx, "/secret.txt", false).await.unwrap();
    work.unlink(&ctx, "/out", true).await.unwrap();
    work.unlink(&ctx, "/dir-link", true).await.unwrap();
    assert_eq!(text(&ctx, &root, "/secret.txt").await, "secret");
    assert!(root.metadata(&ctx, "/other").await.is_ok());
    assert!(work.metadata(&ctx, "/dir").await.is_ok());
    let removed = work.metadata(&ctx, "/out").await.unwrap_err();
    assert_eq!(removed.kind, ErrorKind::NotFound);

    work.unlink(&ctx, "/dir", false).await.unwrap();
    let removed = work.metadata(&ctx, "/dir").await.unwrap_err();
    assert_eq!(removed.kind, ErrorKind::NotFound);
}

#[tokio::test]
async fn writes_need_a_parent_and_follow_only_inner_file_links() {
    let (ctx, root, work) = fixture().await;
    let write = |path: &'static str| work.write(&ctx, path, b"new", file::WriteMode::Truncate);

    write("/new.txt").await.unwrap();
    assert_eq!(text(&ctx, &root, "/work/new.txt").await, "new");

    write("/inner").await.unwrap();
    assert_eq!(text(&ctx, &work, "/file.txt").await, "new");
    let link = work.metadata(&ctx, "/inner").await.unwrap();
    assert_eq!(link.kind, file::Kind::Symlink);

    let missing = write("/missing/new.txt").await.unwrap_err();
    assert_eq!(missing.kind, ErrorKind::NotFound);
    assert_violation("file parent", write("/file.txt/new.txt").await);
    assert_violation("directory", write("/dir").await);
    assert_violation("directory link", write("/dir-link").await);
    assert_violation("root", write("/").await);
    assert_violation("relative", write("new.txt").await);
    assert_violation("parent", write("/../new.txt").await);

    assert_eq!(
        work.resolve(&ctx, "/inner").await.unwrap(),
        file::Path::parse(&ctx, "/file.txt").unwrap()
    );
    assert_eq!(
        work.resolve(&ctx, "/dir/new.txt").await.unwrap(),
        file::Path::parse(&ctx, "/dir/new.txt").unwrap()
    );
}

#[tokio::test]
async fn paths_and_errors_are_relative_to_the_chroot() {
    let (ctx, _root, work) = fixture().await;
    assert_eq!(
        work.read_link(&ctx, "/inner").await.unwrap().as_str(),
        "/file.txt"
    );
    assert_eq!(
        work.resolve(&ctx, "/dir-link").await.unwrap().as_str(),
        "/dir"
    );

    let missing = work.read(&ctx, "/missing.txt", None).await.unwrap_err();
    assert_eq!(missing.kind, ErrorKind::NotFound);
    assert!(
        missing.internal_message.contains("'/missing.txt'"),
        "{}",
        missing.internal_message
    );
    assert!(!missing.internal_message.contains("/work"));
}

#[tokio::test]
async fn copies_and_renames_inside_the_chroot() {
    let (ctx, _root, work) = fixture().await;
    work.copy(&ctx, "/file.txt", "/dir/copy.txt").await.unwrap();
    assert_eq!(text(&ctx, &work, "/dir/copy.txt").await, "file");

    work.rename(&ctx, "/inner", "/dir/inner").await.unwrap();
    let moved = work.metadata(&ctx, "/dir/inner").await.unwrap();
    assert_eq!(moved.kind, file::Kind::Symlink);
    assert_eq!(text(&ctx, &work, "/dir/inner").await, "file");
    assert_violation("rename root", work.rename(&ctx, "/", "/x").await);
}

const ALL: RangeInclusive<usize> = 0..=usize::MAX;

#[allow(clippy::too_many_arguments)]
async fn find(
    ctx: &AppContext,
    chroot: &Chroot,
    path: &str,
    pattern: Option<&Regex>,
    kinds: &[file::Kind],
    depth: RangeInclusive<usize>,
    exclude: &[&str],
    limit: Option<usize>,
) -> Vec<String> {
    let exclude: Vec<String> = exclude
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    chroot
        .find(ctx, path, pattern, kinds, depth, &exclude, limit)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.path.to_string())
        .collect()
}

#[tokio::test]
async fn finds_without_following_symlinks() {
    let (ctx, _root, work) = fixture().await;
    work.write(&ctx, "/dir/deep.txt", b"", file::WriteMode::Truncate)
        .await
        .unwrap();

    assert_eq!(
        find(&ctx, &work, "/", None, &[], ALL, &[], None).await,
        [
            "/",
            "/dir",
            "/dir/deep.txt",
            "/dir-link",
            "/file.txt",
            "/inner",
            "/out",
            "/secret.txt"
        ]
    );
    let excluded = find(&ctx, &work, "/", None, &[], ALL, &["dir", "out"], None).await;
    assert_eq!(excluded.len(), 5);
    let listing = find(&ctx, &work, "/dir", None, &[], 1..=1, &[], None).await;
    assert_eq!(listing, ["/dir/deep.txt"]);
    let only_root = find(&ctx, &work, "/", None, &[], 0..=0, &[], None).await;
    assert_eq!(only_root, ["/"]);
    let through_link = find(&ctx, &work, "/dir-link", None, &[], ALL, &[], None).await;
    assert_eq!(through_link, ["/dir", "/dir/deep.txt"]);
}

#[tokio::test]
async fn finds_by_pattern_kind_and_limit() {
    let (ctx, _root, work) = fixture().await;
    let symlinks = find(
        &ctx,
        &work,
        "/",
        None,
        &[file::Kind::Symlink],
        ALL,
        &[],
        None,
    )
    .await;
    assert_eq!(symlinks, ["/dir-link", "/inner", "/out", "/secret.txt"]);

    let pattern = Regex::new(r"\.txt$").unwrap();
    let kinds = [file::Kind::File, file::Kind::Symlink];
    let first = find(&ctx, &work, "/", Some(&pattern), &kinds, ALL, &[], Some(1)).await;
    assert_eq!(first, ["/file.txt"]);
    let missing = work.find(&ctx, "/missing", None, &[], ALL, &[], None).await;
    assert_eq!(missing.unwrap_err().kind, ErrorKind::NotFound);
}

const MODES: [file::WriteMode; 4] = [
    file::WriteMode::Truncate,
    file::WriteMode::Append,
    file::WriteMode::At(0),
    file::WriteMode::CreateNew,
];

/// Every path of `root` with the content of files and the target of links.
async fn snapshot(ctx: &AppContext, root: &Chroot) -> Vec<(String, Option<String>)> {
    let mut snapshot = vec![];
    for entry in root
        .find(ctx, "/", None, &[], ALL, &[], None)
        .await
        .unwrap()
    {
        let path = entry.path.as_str();
        let content = match entry.metadata.kind {
            file::Kind::File => Some(text(ctx, root, path).await),
            file::Kind::Symlink => Some(root.read_link(ctx, path).await.unwrap().to_string()),
            _ => None,
        };
        snapshot.push((path.to_string(), content));
    }
    snapshot
}

const OPERATIONS: [&str; 18] = [
    "metadata",
    "read",
    "read_to_string",
    "write",
    "resolve",
    "read_dir",
    "mkdir",
    "mkdir -p",
    "copy from",
    "copy to",
    "rename from",
    "rename to",
    "symlink target",
    "symlink link",
    "read_link",
    "unlink",
    "materialize",
    "find",
];

/// Runs one of `OPERATIONS` on `path`, with `/file.txt` and `/new` as the
/// other path of operations that take two.
async fn operate(
    ctx: &AppContext,
    chroot: &Chroot,
    operation: &str,
    path: &str,
) -> Result<(), AppError> {
    match operation {
        "metadata" => chroot.metadata(ctx, path).await.map(drop),
        "read" => chroot.read(ctx, path, None).await.map(drop),
        "read_to_string" => chroot.read_to_string(ctx, path).await.map(drop),
        "write" => {
            chroot
                .write(ctx, path, b"x", file::WriteMode::Truncate)
                .await
        }
        "resolve" => chroot.resolve(ctx, path).await.map(drop),
        "read_dir" => chroot.read_dir(ctx, path).await.map(drop),
        "mkdir" => chroot.mkdir(ctx, path, false).await,
        "mkdir -p" => chroot.mkdir(ctx, path, true).await,
        "copy from" => chroot.copy(ctx, path, "/new").await,
        "copy to" => chroot.copy(ctx, "/file.txt", path).await,
        "rename from" => chroot.rename(ctx, path, "/new").await,
        "rename to" => chroot.rename(ctx, "/file.txt", path).await,
        "symlink target" => chroot.symlink(ctx, path, "/new").await,
        "symlink link" => chroot.symlink(ctx, "/file.txt", path).await,
        "read_link" => chroot.read_link(ctx, path).await.map(drop),
        "unlink" => chroot.unlink(ctx, path, true).await,
        "materialize" => chroot.materialize(ctx, path).await.map(drop),
        "find" => chroot
            .find(ctx, path, None, &[], ALL, &[], None)
            .await
            .map(drop),
        operation => panic!("unknown operation {operation}"),
    }
}

#[tokio::test]
async fn every_operation_rejects_invalid_paths() {
    let (ctx, root, work) = fixture().await;
    let before = snapshot(&ctx, &root).await;
    for operation in OPERATIONS {
        for path in [
            "relative",
            "",
            "/../secret.txt",
            "/dir/../../secret.txt",
            "/a\0b",
        ] {
            let what = format!("{operation} {path:?}");
            assert_violation(&what, operate(&ctx, &work, operation, path).await);
        }
    }
    assert_eq!(snapshot(&ctx, &root).await, before);
}

#[tokio::test]
async fn escaping_leaf_links_are_never_followed() {
    let (ctx, root, work) = fixture().await;
    for mode in MODES {
        let written = work.write(&ctx, "/secret.txt", b"x", mode).await;
        assert_refused(&format!("write {mode:?}"), written);
    }
    assert_refused("copy to", work.copy(&ctx, "/file.txt", "/secret.txt").await);
    assert_refused(
        "copy from",
        work.copy(&ctx, "/secret.txt", "/copy.txt").await,
    );

    work.rename(&ctx, "/secret.txt", "/dir/moved")
        .await
        .unwrap();
    let moved = work.metadata(&ctx, "/dir/moved").await.unwrap();
    assert_eq!(moved.kind, file::Kind::Symlink);
    assert_refused("read moved", work.read(&ctx, "/dir/moved", None).await);
    assert_refused(
        "write moved",
        work.write(&ctx, "/dir/moved", b"x", file::WriteMode::Truncate)
            .await,
    );

    work.rename(&ctx, "/file.txt", "/dir/moved").await.unwrap();
    let replaced = work.metadata(&ctx, "/dir/moved").await.unwrap();
    assert_eq!(replaced.kind, file::Kind::File);
    assert_eq!(text(&ctx, &work, "/dir/moved").await, "file");

    work.unlink(&ctx, "/out", true).await.unwrap();
    assert_eq!(text(&ctx, &root, "/secret.txt").await, "secret");
    assert_eq!(
        root.read_dir(&ctx, "/other").await.unwrap(),
        [] as [app_domains::core::models::file::Entry; 0]
    );
}

#[tokio::test]
async fn links_to_a_sibling_with_a_common_prefix_are_refused() {
    let (ctx, root, work) = fixture().await;
    root.mkdir(&ctx, "/work2", false).await.unwrap();
    root.write(&ctx, "/work2/s.txt", b"sibling", file::WriteMode::Truncate)
        .await
        .unwrap();
    root.symlink(&ctx, "/work2", "/work/sibling").await.unwrap();
    root.mkdir(&ctx, "/work2/dir", false).await.unwrap();
    root.symlink(&ctx, "/work2/s.txt", "/work/sibling.txt")
        .await
        .unwrap();
    let before = snapshot(&ctx, &root).await;

    for operation in OPERATIONS {
        let result = operate(&ctx, &work, operation, "/sibling/s.txt").await;
        assert_refused(operation, result);
        let result = operate(&ctx, &work, operation, "/sibling/dir/new").await;
        assert_refused(&format!("{operation} below the sibling"), result);
    }
    let resolved = work.resolve(&ctx, "/sibling").await.unwrap();
    assert_eq!(resolved.as_str(), "/sibling");
    for operation in ["read", "read_dir", "find", "materialize"] {
        let result = operate(&ctx, &work, operation, "/sibling").await;
        assert_refused(operation, result);
    }
    for mode in MODES {
        let written = work.write(&ctx, "/sibling.txt", b"x", mode).await;
        assert_refused(&format!("write {mode:?}"), written);
    }
    assert_refused(
        "copy from",
        work.copy(&ctx, "/sibling.txt", "/copy.txt").await,
    );
    assert_refused(
        "copy to",
        work.copy(&ctx, "/file.txt", "/sibling.txt").await,
    );
    assert_refused("read_link", work.read_link(&ctx, "/sibling").await);
    assert_refused(
        "symlink target",
        work.symlink(&ctx, "/sibling", "/again").await,
    );
    assert_eq!(snapshot(&ctx, &root).await, before);
}

#[tokio::test]
async fn dangling_links_are_not_written_through() {
    let (ctx, root, work) = fixture().await;
    root.symlink(&ctx, "/missing.txt", "/work/dangling-out")
        .await
        .unwrap();
    root.symlink(&ctx, "/work/missing.txt", "/work/dangling-in")
        .await
        .unwrap();

    for mode in MODES {
        let written = work.write(&ctx, "/dangling-in", b"x", mode).await;
        assert!(written.is_err(), "write {mode:?}");
    }
    assert!(work.copy(&ctx, "/file.txt", "/dangling-in").await.is_err());
    assert!(work.mkdir(&ctx, "/dangling-in", false).await.is_err());
    assert!(work.mkdir(&ctx, "/dangling-in/sub", true).await.is_err());
    let resolved = work.resolve(&ctx, "/dangling-in").await.unwrap();
    assert_eq!(resolved.as_str(), "/dangling-in");
    let missing = root.metadata(&ctx, "/work/missing.txt").await.unwrap_err();
    assert_eq!(missing.kind, ErrorKind::NotFound);

    for appeared in [false, true] {
        if appeared {
            root.write(&ctx, "/missing.txt", b"appeared", file::WriteMode::Truncate)
                .await
                .unwrap();
        }
        for mode in MODES {
            let written = work.write(&ctx, "/dangling-out", b"x", mode).await;
            assert_refused(&format!("write {mode:?} {appeared}"), written);
        }
        let copied = work.copy(&ctx, "/file.txt", "/dangling-out").await;
        assert_refused("copy to", copied);
        assert_refused("read", work.read(&ctx, "/dangling-out", None).await);
        let resolved = work.resolve(&ctx, "/dangling-out").await.unwrap();
        assert_eq!(resolved.as_str(), "/dangling-out");
        assert_refused("mkdir -p", work.mkdir(&ctx, "/dangling-out", true).await);
        let below = work.mkdir(&ctx, "/dangling-out/sub", true).await;
        assert_refused("mkdir -p below", below);
        assert!(work.mkdir(&ctx, "/dangling-out", false).await.is_err());
        if !appeared {
            let missing = root.metadata(&ctx, "/missing.txt").await.unwrap_err();
            assert_eq!(missing.kind, ErrorKind::NotFound);
        }
    }
    assert_eq!(text(&ctx, &root, "/missing.txt").await, "appeared");
}

#[tokio::test]
async fn a_replaced_root_is_not_followed() {
    let (ctx, root, work) = fixture().await;
    root.rename(&ctx, "/work", "/moved").await.unwrap();
    root.symlink(&ctx, "/other", "/work").await.unwrap();
    root.mkdir(&ctx, "/other/dir", false).await.unwrap();
    root.write(&ctx, "/other/file.txt", b"other", file::WriteMode::Truncate)
        .await
        .unwrap();
    let before = snapshot(&ctx, &root).await;

    for operation in OPERATIONS {
        for path in ["/", "/file.txt", "/dir", "/new", "/new/deeper"] {
            let what = format!("{operation} {path}");
            assert_refused(&what, operate(&ctx, &work, operation, path).await);
        }
    }
    assert_eq!(snapshot(&ctx, &root).await, before);
}

#[tokio::test]
async fn recursive_unlink_keeps_link_targets() {
    let (ctx, root, work) = fixture().await;
    root.write(&ctx, "/other/kept.txt", b"kept", file::WriteMode::Truncate)
        .await
        .unwrap();
    for dir in ["/work/dir", "/work/nested/deeper"] {
        root.mkdir(&ctx, dir, true).await.unwrap();
        for (target, name) in [
            ("/other", "out"),
            ("/work", "chroot"),
            ("/secret.txt", "secret"),
        ] {
            root.symlink(&ctx, target, &format!("{dir}/{name}"))
                .await
                .unwrap();
        }
    }

    work.unlink(&ctx, "/dir", true).await.unwrap();
    let removed = work.metadata(&ctx, "/dir").await.unwrap_err();
    assert_eq!(removed.kind, ErrorKind::NotFound);
    assert_eq!(text(&ctx, &work, "/file.txt").await, "file");
    assert!(work.metadata(&ctx, "/nested/deeper/chroot").await.is_ok());

    root.unlink(&ctx, "/work", true).await.unwrap();
    let removed = root.metadata(&ctx, "/work").await.unwrap_err();
    assert_eq!(removed.kind, ErrorKind::NotFound);
    assert_eq!(text(&ctx, &root, "/other/kept.txt").await, "kept");
    assert_eq!(text(&ctx, &root, "/secret.txt").await, "secret");
}

#[tokio::test]
async fn new_creates_nothing_for_invalid_paths() {
    let ctx = context();
    let memory = memory(&ctx).await;
    let root = chroot(&ctx, &memory, "/", false).await.unwrap();
    let before = snapshot(&ctx, &root).await;
    for path in [
        "/runs/../x",
        "/work/..",
        "/work/dir/../../escape",
        "/runs/trace/..",
        "runs/trace",
        "",
        "/runs\0trace",
    ] {
        assert_violation(path, chroot(&ctx, &memory, path, true).await);
    }
    assert_eq!(snapshot(&ctx, &root).await, before);
}

#[tokio::test]
async fn errors_do_not_reveal_paths_outside() {
    let (ctx, root, work) = fixture().await;
    root.mkdir(&ctx, "/work2", false).await.unwrap();
    root.symlink(&ctx, "/work2", "/work/sibling").await.unwrap();

    let errors = [
        work.read(&ctx, "/secret.txt", None).await.map(drop),
        work.read(&ctx, "/out/x", None).await.map(drop),
        work.read(&ctx, "/sibling/x", None).await.map(drop),
        work.read(&ctx, "/missing", None).await.map(drop),
        work.read(&ctx, "/dir", None).await.map(drop),
        work.read_dir(&ctx, "/out").await.map(drop),
        work.read_dir(&ctx, "/file.txt").await.map(drop),
        work.write(&ctx, "/out/x", b"x", file::WriteMode::Truncate)
            .await,
        work.write(&ctx, "/secret.txt", b"x", file::WriteMode::CreateNew)
            .await,
        work.write(&ctx, "/file.txt", b"x", file::WriteMode::CreateNew)
            .await,
        work.write(&ctx, "/dir", b"x", file::WriteMode::Truncate)
            .await,
        work.mkdir(&ctx, "/out/x", true).await,
        work.mkdir(&ctx, "/dir", false).await,
        work.mkdir(&ctx, "/file.txt/x", true).await,
        work.copy(&ctx, "/secret.txt", "/c").await,
        work.copy(&ctx, "/file.txt", "/out/c").await,
        work.copy(&ctx, "/dir", "/c").await,
        work.rename(&ctx, "/file.txt", "/out/x").await,
        work.rename(&ctx, "/missing", "/x").await,
        work.symlink(&ctx, "/out", "/l").await,
        work.symlink(&ctx, "/file.txt", "/dir").await,
        work.read_link(&ctx, "/out").await.map(drop),
        work.read_link(&ctx, "/file.txt").await.map(drop),
        work.unlink(&ctx, "/dir-link/missing", false).await,
        work.materialize(&ctx, "/dir").await.map(drop),
        work.find(&ctx, "/sibling", None, &[], ALL, &[], None)
            .await
            .map(drop),
        work.resolve(&ctx, "/out/x").await.map(drop),
    ];
    for (index, result) in errors.into_iter().enumerate() {
        let message = result.unwrap_err().internal_message;
        for outside in ["/work", "/other"] {
            assert!(!message.contains(outside), "{index}: {message}");
        }
    }
}

#[tokio::test]
async fn symlink_loops_fail() {
    let ctx = context();
    let memory = memory(&ctx).await;
    for (target, link) in [
        ("/work/loop-b", "/work/loop-a"),
        ("/work/loop-a", "/work/loop-b"),
    ] {
        let (target, link) = (
            file::Path::parse(&ctx, target),
            file::Path::parse(&ctx, link),
        );
        memory
            .symlink(&ctx, &target.unwrap(), &link.unwrap())
            .await
            .unwrap();
    }
    let work = chroot(&ctx, &memory, "/work", false).await.unwrap();

    assert!(work.read(&ctx, "/loop-a", None).await.is_err());
    assert!(work.read_dir(&ctx, "/loop-a").await.is_err());
    let resolved = work.resolve(&ctx, "/loop-a").await.unwrap();
    assert_eq!(resolved.as_str(), "/loop-a");
    assert!(work.mkdir(&ctx, "/loop-a/x", true).await.is_err());
    assert!(work.copy(&ctx, "/file.txt", "/loop-a").await.is_err());
    for mode in MODES {
        let written = work.write(&ctx, "/loop-a", b"x", mode).await;
        assert!(written.is_err(), "write {mode:?}");
    }
    assert!(chroot(&ctx, &memory, "/work/loop-a", true).await.is_err());

    let links = find(
        &ctx,
        &work,
        "/",
        None,
        &[file::Kind::Symlink],
        ALL,
        &[],
        None,
    )
    .await;
    assert_eq!(links, ["/dir-link", "/loop-a", "/loop-b"]);
    work.unlink(&ctx, "/loop-a", false).await.unwrap();
    assert_eq!(
        work.metadata(&ctx, "/loop-b").await.unwrap().kind,
        file::Kind::Symlink
    );
}

#[tokio::test]
async fn errors_do_not_tell_whether_something_outside_exists() {
    let (ctx, root, work) = fixture().await;
    root.mkdir(&ctx, "/other/dir", false).await.unwrap();
    root.write(&ctx, "/other/kept.txt", b"kept", file::WriteMode::Truncate)
        .await
        .unwrap();
    root.symlink(&ctx, "/gone.txt", "/work/gone").await.unwrap();
    let before = snapshot(&ctx, &root).await;

    let below = [
        ("/out/kept.txt", "/out/missing.txt"),
        ("/out/dir/x", "/out/nodir/x"),
        ("/out/kept.txt/x", "/out/missing.txt/x"),
        ("/secret.txt/x", "/gone/x"),
    ];
    let leaf_operations = [
        "read",
        "read_to_string",
        "write",
        "read_dir",
        "mkdir -p",
        "copy from",
        "copy to",
        "symlink target",
        "find",
        "materialize",
    ];
    let leaf = [("/secret.txt", "/gone")];
    let cases = below
        .iter()
        .flat_map(|pair| OPERATIONS.iter().map(move |operation| (pair, operation)))
        .chain(leaf.iter().flat_map(|pair| {
            leaf_operations
                .iter()
                .map(move |operation| (pair, operation))
        }));
    for ((existing, missing), operation) in cases {
        let mut keys = vec![];
        for path in [existing, missing] {
            let result = operate(&ctx, &work, operation, path).await;
            keys.push(result.as_ref().map_err(|e| e.translation_key).err());
            assert_refused(&format!("{operation} {path}"), result);
        }
        assert_eq!(keys[0], keys[1], "{operation} {existing} {missing}");
    }
    assert_eq!(snapshot(&ctx, &root).await, before);
}
