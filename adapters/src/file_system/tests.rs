// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use std::{
    ops::RangeInclusive,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;

use super::{Disk, Memory, disk::helpers::find::find_by_walking};
use crate::{configuration::ConfigurationMap, context::ContextStandard};
use app_domains::core::{
    models::{
        AppContext, AppError, ErrorKind,
        file::{self, Chroot},
    },
    traits::FileSystem,
};

pub(super) fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

pub(super) fn path(value: &str) -> file::Path {
    file::Path::parse(&context(), value).unwrap()
}

async fn text(fs: &dyn FileSystem, value: &str) -> String {
    String::from_utf8(fs.read(&context(), &path(value), None).await.unwrap()).unwrap()
}

/// The kind of `value`, `None` if it doesn't exist.
async fn kind(fs: &dyn FileSystem, value: &str) -> Option<file::Kind> {
    match fs.metadata(&context(), &path(value)).await {
        Err(err) if err.kind == ErrorKind::NotFound => None,
        metadata => Some(metadata.unwrap().kind),
    }
}

async fn write(fs: &dyn FileSystem, value: &str, data: &str) {
    fs.write(
        &context(),
        &path(value),
        data.as_bytes(),
        file::WriteMode::Truncate,
    )
    .await
    .unwrap();
}

async fn reads_and_writes(fs: &dyn FileSystem) {
    let ctx = context();
    let file = path("/a.txt");
    write(fs, "/a.txt", "hello world").await;
    assert_eq!(text(fs, "/a.txt").await, "hello world");

    let read = |range| fs.read(&ctx, &file, Some(range));
    assert_eq!(read(0..5).await.unwrap(), b"hello");
    assert_eq!(read(6..100).await.unwrap(), b"world");
    assert_eq!(read(50..60).await.unwrap(), b"");

    fs.write(&ctx, &file, b" !", file::WriteMode::Append)
        .await
        .unwrap();
    assert_eq!(text(fs, "/a.txt").await, "hello world !");
    fs.write(&ctx, &file, b"J", file::WriteMode::At(0))
        .await
        .unwrap();
    assert_eq!(text(fs, "/a.txt").await, "Jello world !");
    fs.write(&ctx, &file, b"x", file::WriteMode::At(15))
        .await
        .unwrap();
    assert_eq!(
        fs.read(&ctx, &file, None).await.unwrap(),
        b"Jello world !\0\0x"
    );
    write(fs, "/a.txt", "short").await;
    assert_eq!(text(fs, "/a.txt").await, "short");

    let created = fs
        .write(&ctx, &file, b"new", file::WriteMode::CreateNew)
        .await;
    assert_eq!(created.unwrap_err().kind, ErrorKind::Conflict);
    fs.write(&ctx, &path("/b.txt"), b"new", file::WriteMode::CreateNew)
        .await
        .unwrap();
    fs.write(&ctx, &path("/c.txt"), b"c", file::WriteMode::Append)
        .await
        .unwrap();
    assert_eq!(text(fs, "/c.txt").await, "c");

    let missing = fs.read(&ctx, &path("/missing.txt"), None).await;
    assert_eq!(missing.unwrap_err().kind, ErrorKind::NotFound);
    let no_parent = fs
        .write(
            &ctx,
            &path("/missing/a.txt"),
            b"x",
            file::WriteMode::Truncate,
        )
        .await;
    assert_eq!(no_parent.unwrap_err().kind, ErrorKind::NotFound);
}

async fn lists_and_describes(fs: &dyn FileSystem) {
    let ctx = context();
    fs.mkdir(&ctx, &path("/list"), false).await.unwrap();
    write(fs, "/list/b", "bb").await;
    write(fs, "/list/a", "a").await;
    fs.mkdir(&ctx, &path("/list/c"), false).await.unwrap();
    write(fs, "/list/c/nested", "").await;

    let entries = fs.read_dir(&ctx, &path("/list")).await.unwrap();
    let names: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry.path.file_name())
        .collect();
    assert_eq!(names, ["a", "b", "c"]);
    assert_eq!(entries[1].metadata.kind, file::Kind::File);
    assert_eq!(entries[1].metadata.size, 2);
    assert_eq!(entries[2].metadata.kind, file::Kind::Directory);

    assert_eq!(kind(fs, "/").await, Some(file::Kind::Directory));
    assert_eq!(kind(fs, "/list/a").await, Some(file::Kind::File));
    assert_eq!(kind(fs, "/list/missing").await, None);
    assert_eq!(kind(fs, "/missing/deeper").await, None);
    assert!(fs.read_dir(&ctx, &path("/list/a")).await.is_err());
}

async fn modified(fs: &dyn FileSystem, value: &str) -> u64 {
    fs.metadata(&context(), &path(value))
        .await
        .unwrap()
        .modified
}

async fn stamps_modified_times(fs: &dyn FileSystem) {
    let ctx = context();
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 60;
    fs.mkdir(&ctx, &path("/stamp"), false).await.unwrap();
    write(fs, "/stamp/file", "one").await;
    fs.symlink(&ctx, &path("/stamp/file"), &path("/stamp/link"))
        .await
        .unwrap();
    for value in ["/stamp", "/stamp/file", "/stamp/link"] {
        assert!(modified(fs, value).await > start, "{value}");
    }

    let written = modified(fs, "/stamp/file").await;
    write(fs, "/stamp/file", "two").await;
    assert!(modified(fs, "/stamp/file").await >= written);

    let before = modified(fs, "/stamp/file").await;
    fs.rename(&ctx, &path("/stamp/file"), &path("/stamp/renamed"))
        .await
        .unwrap();
    assert_eq!(modified(fs, "/stamp/renamed").await, before);
}

async fn creates_directories(fs: &dyn FileSystem) {
    let ctx = context();
    let missing_parent = fs.mkdir(&ctx, &path("/x/y"), false).await;
    assert_eq!(missing_parent.unwrap_err().kind, ErrorKind::NotFound);

    fs.mkdir(&ctx, &path("/x/y/z"), true).await.unwrap();
    fs.mkdir(&ctx, &path("/x/y/z"), true).await.unwrap();
    assert_eq!(kind(fs, "/x/y/z").await, Some(file::Kind::Directory));

    let existing = fs.mkdir(&ctx, &path("/x"), false).await;
    assert_eq!(existing.unwrap_err().kind, ErrorKind::Conflict);
}

async fn copies_and_renames(fs: &dyn FileSystem) {
    let ctx = context();
    fs.mkdir(&ctx, &path("/move/inner"), true).await.unwrap();
    write(fs, "/move/inner/file", "content").await;

    fs.copy(&ctx, &path("/move/inner/file"), &path("/move/copy"))
        .await
        .unwrap();
    assert_eq!(text(fs, "/move/copy").await, "content");
    assert_eq!(text(fs, "/move/inner/file").await, "content");
    assert!(
        fs.copy(&ctx, &path("/move/inner"), &path("/move/dir-copy"))
            .await
            .is_err()
    );

    fs.rename(&ctx, &path("/move/copy"), &path("/move/renamed"))
        .await
        .unwrap();
    assert_eq!(kind(fs, "/move/copy").await, None);
    assert_eq!(text(fs, "/move/renamed").await, "content");

    fs.rename(&ctx, &path("/move/inner"), &path("/moved"))
        .await
        .unwrap();
    assert_eq!(kind(fs, "/move/inner").await, None);
    assert_eq!(text(fs, "/moved/file").await, "content");
}

async fn links(fs: &dyn FileSystem) {
    let ctx = context();
    fs.mkdir(&ctx, &path("/target/dir"), true).await.unwrap();
    fs.mkdir(&ctx, &path("/links"), false).await.unwrap();
    write(fs, "/target/dir/file", "linked").await;

    fs.symlink(&ctx, &path("/target/dir/file"), &path("/links/file"))
        .await
        .unwrap();
    fs.symlink(&ctx, &path("/target/dir"), &path("/links/dir"))
        .await
        .unwrap();
    fs.symlink(&ctx, &path("/links/dir"), &path("/links/chain"))
        .await
        .unwrap();

    assert_eq!(
        fs.read_link(&ctx, &path("/links/file")).await.unwrap(),
        path("/target/dir/file")
    );
    assert_eq!(kind(fs, "/links/file").await, Some(file::Kind::Symlink));
    assert_eq!(text(fs, "/links/file").await, "linked");
    assert_eq!(text(fs, "/links/chain/file").await, "linked");
    assert_eq!(
        fs.canonicalize(&ctx, &path("/links/chain/file"))
            .await
            .unwrap(),
        path("/target/dir/file")
    );
    assert_eq!(
        fs.canonicalize(&ctx, &path("/")).await.unwrap(),
        file::Path::root()
    );
    let existing = fs
        .symlink(&ctx, &path("/target"), &path("/links/file"))
        .await;
    assert_eq!(existing.unwrap_err().kind, ErrorKind::Conflict);

    fs.symlink(&ctx, &path("/nowhere"), &path("/links/dangling"))
        .await
        .unwrap();
    assert_eq!(kind(fs, "/links/dangling").await, Some(file::Kind::Symlink));
    let dangling = fs.canonicalize(&ctx, &path("/links/dangling")).await;
    assert_eq!(dangling.unwrap_err().kind, ErrorKind::NotFound);

    let listed = fs.read_dir(&ctx, &path("/links")).await.unwrap();
    assert_eq!(kinds(&listed), [file::Kind::Symlink; 4]);
}

fn kinds(entries: &[file::Entry]) -> Vec<file::Kind> {
    entries.iter().map(|entry| entry.metadata.kind).collect()
}

async fn unlinks_without_following(fs: &dyn FileSystem) {
    let ctx = context();
    fs.mkdir(&ctx, &path("/keep/dir"), true).await.unwrap();
    fs.mkdir(&ctx, &path("/remove"), false).await.unwrap();
    write(fs, "/keep/dir/file", "kept").await;
    fs.symlink(&ctx, &path("/keep/dir/file"), &path("/remove/file"))
        .await
        .unwrap();
    fs.symlink(&ctx, &path("/keep/dir"), &path("/remove/dir"))
        .await
        .unwrap();

    fs.unlink(&ctx, &path("/remove/file"), false).await.unwrap();
    fs.unlink(&ctx, &path("/remove/dir"), true).await.unwrap();
    assert_eq!(kind(fs, "/remove/file").await, None);
    assert_eq!(kind(fs, "/remove/dir").await, None);
    assert_eq!(text(fs, "/keep/dir/file").await, "kept");

    fs.mkdir(&ctx, &path("/remove/nested"), false)
        .await
        .unwrap();
    for (target, link) in [
        ("/keep/dir", "/remove/nested/dir"),
        ("/keep", "/remove/keep"),
    ] {
        fs.symlink(&ctx, &path(target), &path(link)).await.unwrap();
    }
    fs.unlink(&ctx, &path("/remove"), true).await.unwrap();
    assert_eq!(kind(fs, "/remove").await, None);
    assert_eq!(text(fs, "/keep/dir/file").await, "kept");

    let not_empty = fs.unlink(&ctx, &path("/keep"), false).await;
    assert!(not_empty.is_err());
    fs.unlink(&ctx, &path("/keep"), true).await.unwrap();
    assert_eq!(kind(fs, "/keep").await, None);

    assert!(fs.unlink(&ctx, &file::Path::root(), true).await.is_err());
    assert_eq!(kind(fs, "/").await, Some(file::Kind::Directory));
    let missing = fs.unlink(&ctx, &path("/missing"), false).await;
    assert_eq!(missing.unwrap_err().kind, ErrorKind::NotFound);
}

/// `a` ↔ `b` and `self` → `self`: everything through them fails, nothing hangs.
async fn loops(fs: &(dyn FileSystem + Sync)) {
    let ctx = context();
    fs.mkdir(&ctx, &path("/loops"), false).await.unwrap();
    for (target, link) in [("b", "a"), ("a", "b"), ("self", "self")] {
        let (target, link) = (format!("/loops/{target}"), format!("/loops/{link}"));
        fs.symlink(&ctx, &path(&target), &path(&link))
            .await
            .unwrap();
    }

    for link in ["/loops/a", "/loops/self"] {
        let (at, below) = (path(link), path(&format!("{link}/x")));
        assert!(fs.canonicalize(&ctx, &at).await.is_err(), "{}", link);
        assert!(fs.canonicalize(&ctx, &below).await.is_err(), "{}", link);
        assert!(fs.read(&ctx, &at, None).await.is_err(), "{}", link);
        assert!(fs.read_dir(&ctx, &at).await.is_err(), "{}", link);
        let written = fs.write(&ctx, &at, b"x", file::WriteMode::Truncate).await;
        assert!(written.is_err(), "{}", link);
        assert!(fs.mkdir(&ctx, &below, true).await.is_err(), "{}", link);
        assert_eq!(kind(fs, link).await, Some(file::Kind::Symlink));
    }

    let listed = fs.read_dir(&ctx, &path("/loops")).await.unwrap();
    assert_eq!(kinds(&listed), [file::Kind::Symlink; 3]);
    let all = 0..=usize::MAX;
    let found = fs
        .find(&ctx, &path("/loops"), None, &[], all, &[], None)
        .await;
    assert_eq!(
        paths(found.unwrap()),
        ["/loops", "/loops/a", "/loops/b", "/loops/self"]
    );
    fs.unlink(&ctx, &path("/loops"), true).await.unwrap();
    assert_eq!(kind(fs, "/loops").await, None);
}

/// A tree for `find`: `/tree` with `a.txt`, `a-b/c.txt`, `a/b.md`,
/// `a/deep/d.txt`, `skip/e.txt` and the symlink `link` to `a`.
async fn tree(fs: &dyn FileSystem) {
    let ctx = context();
    for dir in ["/tree/a/deep", "/tree/a-b", "/tree/skip"] {
        fs.mkdir(&ctx, &path(dir), true).await.unwrap();
    }
    for file in [
        "/tree/a.txt",
        "/tree/a-b/c.txt",
        "/tree/a/b.md",
        "/tree/a/deep/d.txt",
        "/tree/skip/e.txt",
    ] {
        write(fs, file, "").await;
    }
    fs.symlink(&ctx, &path("/tree/a"), &path("/tree/link"))
        .await
        .unwrap();
}

/// `find` arguments: pattern, kinds, depth, exclude, limit.
type Find = (
    Option<Regex>,
    Vec<file::Kind>,
    RangeInclusive<usize>,
    Vec<String>,
    Option<usize>,
);

/// What `find` should return for each set of arguments on `tree`.
fn finds_on_tree() -> Vec<(Find, Vec<&'static str>)> {
    let all = 0..=usize::MAX;
    vec![
        (
            (None, vec![], all.clone(), vec![], None),
            vec![
                "/tree",
                "/tree/a",
                "/tree/a/b.md",
                "/tree/a/deep",
                "/tree/a/deep/d.txt",
                "/tree/a-b",
                "/tree/a-b/c.txt",
                "/tree/a.txt",
                "/tree/link",
                "/tree/skip",
                "/tree/skip/e.txt",
            ],
        ),
        (
            (None, vec![], 1..=1, vec![], None),
            vec![
                "/tree/a",
                "/tree/a-b",
                "/tree/a.txt",
                "/tree/link",
                "/tree/skip",
            ],
        ),
        (
            (
                Some(Regex::new(r"\.txt$").unwrap()),
                vec![file::Kind::File],
                all,
                vec!["skip".to_string()],
                None,
            ),
            vec!["/tree/a/deep/d.txt", "/tree/a-b/c.txt", "/tree/a.txt"],
        ),
        (
            (
                None,
                vec![file::Kind::Directory, file::Kind::Symlink],
                0..=1,
                vec![],
                Some(3),
            ),
            vec!["/tree", "/tree/a", "/tree/a-b"],
        ),
    ]
}

fn paths(entries: Vec<file::Entry>) -> Vec<String> {
    entries
        .into_iter()
        .map(|entry| entry.path.to_string())
        .collect()
}

async fn finds(fs: &(dyn FileSystem + Sync)) {
    let ctx = context();
    tree(fs).await;
    let start = path("/tree");

    for ((pattern, kinds, depth, exclude, limit), expected) in finds_on_tree() {
        let pattern = pattern.as_ref();
        let found = fs
            .find(
                &ctx,
                &start,
                pattern,
                &kinds,
                depth.clone(),
                &exclude,
                limit,
            )
            .await;
        assert_eq!(paths(found.unwrap()), expected, "{kinds:?} {depth:?}");
        let walked = find_by_walking(
            fs,
            &ctx,
            &start,
            pattern,
            &kinds,
            depth.clone(),
            &exclude,
            limit,
        )
        .await;
        assert_eq!(
            paths(walked.unwrap()),
            expected,
            "walking {kinds:?} {depth:?}"
        );
    }

    let all = 0..=usize::MAX;
    for single in ["/tree/link", "/tree/a.txt"] {
        let found = fs
            .find(&ctx, &path(single), None, &[], all.clone(), &[], None)
            .await;
        assert_eq!(paths(found.unwrap()), [single]);
    }
    let missing = fs
        .find(&ctx, &path("/missing"), None, &[], all, &[], None)
        .await;
    assert_eq!(missing.unwrap_err().kind, ErrorKind::NotFound);
}

pub(super) async fn conforms(fs: &(dyn FileSystem + Sync)) {
    reads_and_writes(fs).await;
    lists_and_describes(fs).await;
    stamps_modified_times(fs).await;
    creates_directories(fs).await;
    copies_and_renames(fs).await;
    links(fs).await;
    unlinks_without_following(fs).await;
    loops(fs).await;
    finds(fs).await;
}

type Fs = Arc<dyn FileSystem + Send + Sync>;

const MODES: [file::WriteMode; 4] = [
    file::WriteMode::Truncate,
    file::WriteMode::Append,
    file::WriteMode::At(0),
    file::WriteMode::CreateNew,
];

/// A scenario below `base`: the chroot `work` with `file.txt` and `dir`,
/// next to `other`, `secret.txt` and the sibling `work2` with `s.txt`. `work`
/// has the links `out` → `other`, `secret.txt` → `secret.txt`, `sibling` →
/// `work2` and `sibling.txt` → `work2/s.txt`. Returns the chroots of `base`
/// and of `work`.
async fn scenario(fs: &Fs, base: &str) -> (Chroot, Chroot) {
    let ctx = context();
    let outer = Chroot::new(&ctx, fs.clone(), base, true).await.unwrap();
    for dir in ["/work/dir", "/other", "/work2"] {
        outer.mkdir(&ctx, dir, true).await.unwrap();
    }
    for (file, content) in [
        ("/work/file.txt", "file"),
        ("/secret.txt", "secret"),
        ("/work2/s.txt", "sibling"),
    ] {
        outer
            .write(&ctx, file, content.as_bytes(), file::WriteMode::Truncate)
            .await
            .unwrap();
    }
    for (target, link) in [
        ("/other", "/work/out"),
        ("/secret.txt", "/work/secret.txt"),
        ("/work2", "/work/sibling"),
        ("/work2/s.txt", "/work/sibling.txt"),
    ] {
        outer.symlink(&ctx, target, link).await.unwrap();
    }
    let work = Chroot::new(&ctx, fs.clone(), &format!("{base}/work"), false)
        .await
        .unwrap();
    (outer, work)
}

/// Every path of `chroot` with the content of files and the target of links.
async fn snapshot(chroot: &Chroot) -> Vec<(String, Option<String>)> {
    let ctx = context();
    let all = 0..=usize::MAX;
    let mut snapshot = vec![];
    for entry in chroot
        .find(&ctx, "/", None, &[], all, &[], None)
        .await
        .unwrap()
    {
        let path = entry.path.as_str();
        let content = match entry.metadata.kind {
            file::Kind::File => Some(chroot.read_to_string(&ctx, path).await.unwrap()),
            file::Kind::Symlink => Some(chroot.read_link(&ctx, path).await.unwrap().to_string()),
            _ => None,
        };
        snapshot.push((path.to_string(), content));
    }
    snapshot
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

/// The files outside of `work` are unchanged.
async fn assert_outside_unchanged(outer: &Chroot) {
    let ctx = context();
    assert_eq!(
        outer.read_to_string(&ctx, "/secret.txt").await.unwrap(),
        "secret"
    );
    assert_eq!(
        outer.read_to_string(&ctx, "/work2/s.txt").await.unwrap(),
        "sibling"
    );
    assert_eq!(outer.read_dir(&ctx, "/work2").await.unwrap().len(), 1);
}

async fn leaf_links_are_not_followed(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/leaf").await;
    for link in ["/secret.txt", "/sibling.txt"] {
        for mode in MODES {
            let written = work.write(&ctx, link, b"x", mode).await;
            assert_refused(&format!("write {mode:?} {link}"), written);
        }
        assert_refused(link, work.copy(&ctx, "/file.txt", link).await);
        assert_refused(link, work.copy(&ctx, link, "/copy.txt").await);
    }

    work.rename(&ctx, "/secret.txt", "/dir/moved")
        .await
        .unwrap();
    assert!(work.read(&ctx, "/dir/moved", None).await.is_err());
    let written = work
        .write(&ctx, "/dir/moved", b"x", file::WriteMode::Truncate)
        .await;
    assert!(written.is_err());
    work.rename(&ctx, "/file.txt", "/sibling.txt")
        .await
        .unwrap();
    let replaced = work.metadata(&ctx, "/sibling.txt").await.unwrap();
    assert_eq!(replaced.kind, file::Kind::File);
    work.unlink(&ctx, "/out", true).await.unwrap();
    work.unlink(&ctx, "/sibling", false).await.unwrap();

    assert_outside_unchanged(&outer).await;
    assert_eq!(
        outer.read_dir(&ctx, "/other").await.unwrap(),
        [] as [app_domains::core::models::file::Entry; 0]
    );
}

async fn siblings_are_outside(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/sibling").await;
    let all = 0..=usize::MAX;
    let refused = [
        (
            "read",
            work.read(&ctx, "/sibling/s.txt", None).await.map(drop),
        ),
        (
            "metadata",
            work.metadata(&ctx, "/sibling/s.txt").await.map(drop),
        ),
        ("read_dir", work.read_dir(&ctx, "/sibling").await.map(drop)),
        (
            "read_link",
            work.read_link(&ctx, "/sibling").await.map(drop),
        ),
        (
            "write",
            work.write(&ctx, "/sibling/new.txt", b"x", file::WriteMode::Truncate)
                .await,
        ),
        ("mkdir", work.mkdir(&ctx, "/sibling/new", false).await),
        ("mkdir -p", work.mkdir(&ctx, "/sibling/a/b", true).await),
        ("copy to", work.copy(&ctx, "/file.txt", "/sibling/c").await),
        ("copy from", work.copy(&ctx, "/sibling/s.txt", "/c").await),
        (
            "rename to",
            work.rename(&ctx, "/file.txt", "/sibling/m").await,
        ),
        (
            "rename from",
            work.rename(&ctx, "/sibling/s.txt", "/m").await,
        ),
        ("unlink", work.unlink(&ctx, "/sibling/s.txt", false).await),
        ("symlink target", work.symlink(&ctx, "/sibling", "/l").await),
        (
            "symlink in",
            work.symlink(&ctx, "/file.txt", "/sibling/l").await,
        ),
        (
            "find",
            work.find(&ctx, "/sibling", None, &[], all, &[], None)
                .await
                .map(drop),
        ),
        (
            "materialize",
            work.materialize(&ctx, "/sibling").await.map(drop),
        ),
    ];
    for (what, result) in refused {
        assert_refused(what, result);
    }
    let resolved = work.resolve(&ctx, "/sibling").await.unwrap();
    assert_eq!(resolved.as_str(), "/sibling");
    assert_outside_unchanged(&outer).await;
}

async fn dangling_links_create_nothing(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/dangling").await;
    outer
        .symlink(&ctx, "/missing.txt", "/work/dangling-out")
        .await
        .unwrap();
    outer
        .symlink(&ctx, "/work/missing.txt", "/work/dangling-in")
        .await
        .unwrap();

    for mode in MODES {
        let written = work.write(&ctx, "/dangling-in", b"x", mode).await;
        assert!(written.is_err(), "write {mode:?}");
    }
    assert!(work.copy(&ctx, "/file.txt", "/dangling-in").await.is_err());
    assert!(work.mkdir(&ctx, "/dangling-in/sub", true).await.is_err());
    let resolved = work.resolve(&ctx, "/dangling-in").await.unwrap();
    assert_eq!(resolved.as_str(), "/dangling-in");
    let missing = outer.metadata(&ctx, "/work/missing.txt").await.unwrap_err();
    assert_eq!(missing.kind, ErrorKind::NotFound);

    for appeared in [false, true] {
        if appeared {
            outer
                .write(&ctx, "/missing.txt", b"appeared", file::WriteMode::Truncate)
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
        let below = work.mkdir(&ctx, "/dangling-out/sub", true).await;
        assert_refused("mkdir -p below", below);
        if !appeared {
            let missing = outer.metadata(&ctx, "/missing.txt").await.unwrap_err();
            assert_eq!(missing.kind, ErrorKind::NotFound);
        }
    }
    let appeared = outer.read_to_string(&ctx, "/missing.txt").await.unwrap();
    assert_eq!(appeared, "appeared");
}

/// Runs `operation` on `path`, with `/file.txt` and `/new` as the other path.
async fn operate(chroot: &Chroot, operation: &str, path: &str) -> Result<(), AppError> {
    let ctx = context();
    let all = 0..=usize::MAX;
    match operation {
        "metadata" => chroot.metadata(&ctx, path).await.map(drop),
        "read" => chroot.read(&ctx, path, None).await.map(drop),
        "write" => {
            chroot
                .write(&ctx, path, b"x", file::WriteMode::Truncate)
                .await
        }
        "resolve" => chroot.resolve(&ctx, path).await.map(drop),
        "read_dir" => chroot.read_dir(&ctx, path).await.map(drop),
        "mkdir" => chroot.mkdir(&ctx, path, false).await,
        "mkdir -p" => chroot.mkdir(&ctx, path, true).await,
        "copy from" => chroot.copy(&ctx, path, "/new").await,
        "copy to" => chroot.copy(&ctx, "/file.txt", path).await,
        "rename from" => chroot.rename(&ctx, path, "/new").await,
        "rename to" => chroot.rename(&ctx, "/file.txt", path).await,
        "symlink target" => chroot.symlink(&ctx, path, "/new").await,
        "symlink link" => chroot.symlink(&ctx, "/file.txt", path).await,
        "read_link" => chroot.read_link(&ctx, path).await.map(drop),
        "unlink" => chroot.unlink(&ctx, path, true).await,
        "materialize" => chroot.materialize(&ctx, path).await.map(drop),
        "find" => chroot
            .find(&ctx, path, None, &[], all, &[], None)
            .await
            .map(drop),
        operation => panic!("unknown operation {operation}"),
    }
}

/// Paths through links leading outside fail the same way whether something
/// exists there or not.
async fn outside_can_not_be_probed(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/probe").await;
    outer.mkdir(&ctx, "/other/dir", false).await.unwrap();
    outer
        .write(&ctx, "/other/kept.txt", b"kept", file::WriteMode::Truncate)
        .await
        .unwrap();
    outer
        .symlink(&ctx, "/gone.txt", "/work/gone")
        .await
        .unwrap();
    let before = snapshot(&outer).await;

    let operations = [
        "metadata",
        "read",
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
    let below = [
        ("/out/kept.txt", "/out/missing.txt"),
        ("/out/dir/x", "/out/nodir/x"),
        ("/out/kept.txt/x", "/out/missing.txt/x"),
        ("/secret.txt/x", "/gone/x"),
        ("/sibling/s.txt/x", "/sibling/missing/x"),
    ];
    for (existing, missing) in below {
        for operation in operations {
            let mut keys = vec![];
            for path in [existing, missing] {
                let result = operate(&work, operation, path).await;
                keys.push(result.as_ref().map_err(|e| e.translation_key).err());
                assert_refused(&format!("{operation} {path}"), result);
            }
            assert_eq!(keys[0], keys[1], "{operation} {existing} {missing}");
        }
    }
    let leaf_operations = [
        "read",
        "write",
        "read_dir",
        "mkdir -p",
        "copy from",
        "copy to",
        "find",
    ];
    for operation in leaf_operations {
        let mut keys = vec![];
        for path in ["/secret.txt", "/gone"] {
            let result = operate(&work, operation, path).await;
            keys.push(result.as_ref().map_err(|e| e.translation_key).err());
            assert_refused(&format!("{operation} {path}"), result);
        }
        assert_eq!(keys[0], keys[1], "{operation}");
    }
    assert_eq!(snapshot(&outer).await, before);
}

async fn a_replaced_root_is_not_followed(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/replaced").await;
    outer.rename(&ctx, "/work", "/moved").await.unwrap();
    outer.symlink(&ctx, "/other", "/work").await.unwrap();
    outer.mkdir(&ctx, "/other/dir", false).await.unwrap();
    outer
        .write(&ctx, "/other/file.txt", b"other", file::WriteMode::Truncate)
        .await
        .unwrap();
    let before = snapshot(&outer).await;

    let all = 0..=usize::MAX;
    let refused = [
        ("metadata", work.metadata(&ctx, "/").await.map(drop)),
        ("read", work.read(&ctx, "/file.txt", None).await.map(drop)),
        ("read_dir", work.read_dir(&ctx, "/").await.map(drop)),
        ("resolve", work.resolve(&ctx, "/new").await.map(drop)),
        (
            "write",
            work.write(&ctx, "/new.txt", b"x", file::WriteMode::Truncate)
                .await,
        ),
        (
            "overwrite",
            work.write(&ctx, "/file.txt", b"x", file::WriteMode::Truncate)
                .await,
        ),
        ("mkdir", work.mkdir(&ctx, "/new", false).await),
        ("mkdir -p", work.mkdir(&ctx, "/new/deeper", true).await),
        ("mkdir -p root", work.mkdir(&ctx, "/", true).await),
        ("copy", work.copy(&ctx, "/file.txt", "/copy.txt").await),
        ("rename", work.rename(&ctx, "/file.txt", "/moved.txt").await),
        ("symlink", work.symlink(&ctx, "/file.txt", "/link").await),
        ("read_link", work.read_link(&ctx, "/dir").await.map(drop)),
        ("unlink", work.unlink(&ctx, "/file.txt", false).await),
        ("unlink dir", work.unlink(&ctx, "/dir", true).await),
        (
            "find",
            work.find(&ctx, "/", None, &[], all, &[], None)
                .await
                .map(drop),
        ),
        ("materialize", work.materialize(&ctx, "/").await.map(drop)),
    ];
    for (what, result) in refused {
        assert_refused(what, result);
    }
    assert_eq!(snapshot(&outer).await, before);
}

async fn recursive_unlink_keeps_link_targets(fs: &Fs) {
    let ctx = context();
    let (outer, work) = scenario(fs, "/unlink").await;
    outer
        .write(&ctx, "/other/kept.txt", b"kept", file::WriteMode::Truncate)
        .await
        .unwrap();
    for dir in ["/work/dir", "/work/nested/deeper"] {
        outer.mkdir(&ctx, dir, true).await.unwrap();
        for (target, name) in [
            ("/other", "out"),
            ("/work", "chroot"),
            ("/secret.txt", "secret"),
            ("/work2", "sibling"),
        ] {
            let link = format!("{dir}/{name}");
            outer.symlink(&ctx, target, &link).await.unwrap();
        }
    }

    work.unlink(&ctx, "/dir", true).await.unwrap();
    assert_eq!(
        work.metadata(&ctx, "/dir").await.unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert_eq!(
        work.read_to_string(&ctx, "/file.txt").await.unwrap(),
        "file"
    );
    outer.unlink(&ctx, "/work", true).await.unwrap();
    assert_eq!(
        outer.metadata(&ctx, "/work").await.unwrap_err().kind,
        ErrorKind::NotFound
    );
    let kept = outer.read_to_string(&ctx, "/other/kept.txt").await.unwrap();
    assert_eq!(kept, "kept");
    assert_outside_unchanged(&outer).await;
}

async fn loops_fail_through_chroots(fs: &Fs) {
    let ctx = context();
    let (_outer, work) = scenario(fs, "/loops-chroot").await;
    for (target, link) in [("loop-b", "loop-a"), ("loop-a", "loop-b")] {
        let at = |name| path(&format!("/loops-chroot/work/{name}"));
        fs.symlink(&ctx, &at(target), &at(link)).await.unwrap();
    }

    assert!(work.read(&ctx, "/loop-a", None).await.is_err());
    assert!(work.mkdir(&ctx, "/loop-a/x", true).await.is_err());
    for mode in MODES {
        let written = work.write(&ctx, "/loop-a", b"x", mode).await;
        assert!(written.is_err(), "write {mode:?}");
    }
    let all = 0..=usize::MAX;
    let links = work
        .find(&ctx, "/", None, &[file::Kind::Symlink], all, &[], None)
        .await
        .unwrap();
    assert_eq!(links.len(), 6);
}

/// Escapes from a `Chroot` over `fs`, which must be empty.
pub(super) async fn contains(fs: Fs) {
    leaf_links_are_not_followed(&fs).await;
    siblings_are_outside(&fs).await;
    dangling_links_create_nothing(&fs).await;
    outside_can_not_be_probed(&fs).await;
    a_replaced_root_is_not_followed(&fs).await;
    recursive_unlink_keeps_link_targets(&fs).await;
    loops_fail_through_chroots(&fs).await;
}

#[tokio::test]
async fn memory_conforms() {
    conforms(&Memory::new()).await;
}

#[tokio::test]
async fn disk_conforms() {
    let root = TempDir::new().unwrap();
    conforms(&Disk::new(&context(), &root.path().to_string_lossy()).unwrap()).await;
}

#[tokio::test]
async fn memory_contains_chroots() {
    contains(Arc::new(Memory::new())).await;
}

#[tokio::test]
async fn disk_contains_chroots() {
    let root = TempDir::new().unwrap();
    contains(Arc::new(
        Disk::new(&context(), &root.path().to_string_lossy()).unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn memory_cannot_materialize() {
    let Err(error) = Memory::new()
        .materialize(&context(), &file::Path::root())
        .await
    else {
        panic!("memory materialized a directory");
    };
    assert_eq!(error.translation_key, "file_system_unsupported");
}
