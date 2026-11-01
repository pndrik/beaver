// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::core::models::file;

use crate::context;

fn path(value: &str) -> file::Path {
    file::Path::parse(&context(), value).unwrap()
}

#[test]
fn parsing_normalizes() {
    for (input, expected) in [
        ("/", "/"),
        ("//", "/"),
        ("/.", "/"),
        ("/a/./b//c/", "/a/b/c"),
        ("/a..b/..c/d..", "/a..b/..c/d.."),
        ("/.hidden", "/.hidden"),
    ] {
        assert_eq!(path(input).as_str(), expected, "{input}");
    }
}

#[test]
fn parsing_rejects_escapes_and_relative_paths() {
    for input in [
        "",
        "a/b",
        "./a",
        "/..",
        "/a/../b",
        "/a/..",
        "/a/b/../..",
        "/./..",
        "/a\0b",
    ] {
        let err = file::Path::parse(&context(), input).unwrap_err();
        assert_eq!(err.translation_key, "access_violation", "{input:?}");
    }
}

#[test]
fn joins_and_splits_lexically() {
    let ctx = context();
    assert_eq!(path("/a").join(&path("/b/c")), path("/a/b/c"));
    assert_eq!(file::Path::root().join(&path("/b")), path("/b"));
    assert_eq!(path("/a").join(&file::Path::root()), path("/a"));
    assert_eq!(path("/a").child(&ctx, "b").unwrap(), path("/a/b"));
    for name in ["", ".", "..", "a/b", "a\0b"] {
        assert!(path("/a").child(&ctx, name).is_err(), "{}", name);
    }

    assert_eq!(path("/a/b").parent(), Some(path("/a")));
    assert_eq!(path("/a").parent(), Some(file::Path::root()));
    assert_eq!(file::Path::root().parent(), None);
    assert_eq!(path("/a/b").file_name(), Some("b"));
    assert_eq!(file::Path::root().file_name(), None);
    assert_eq!(path("/a/b").components().collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn prefixes_compare_components() {
    assert!(path("/a/b").starts_with(&path("/a")));
    assert!(path("/a").starts_with(&path("/a")));
    assert!(path("/a").starts_with(&file::Path::root()));
    assert!(!path("/ab").starts_with(&path("/a")));
    assert!(!path("/a").starts_with(&path("/a/b")));

    assert_eq!(path("/a/b").strip_prefix(&path("/a")), Some(path("/b")));
    assert_eq!(
        path("/a").strip_prefix(&path("/a")),
        Some(file::Path::root())
    );
    assert_eq!(
        path("/a").strip_prefix(&file::Path::root()),
        Some(path("/a"))
    );
    assert_eq!(path("/ab").strip_prefix(&path("/a")), None);
}

#[test]
fn sorts_by_component() {
    let mut paths = [
        path("/a-b"),
        path("/a/x"),
        path("/b"),
        path("/a"),
        path("/"),
    ];
    paths.sort();
    let sorted: Vec<&str> = paths.iter().map(file::Path::as_str).collect();
    assert_eq!(sorted, ["/", "/a", "/a/x", "/a-b", "/b"]);
}
