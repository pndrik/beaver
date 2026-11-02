// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    collections::HashMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;

use super::tools;
use crate::{configuration::ConfigurationMap, context::ContextStandard, file_system::Disk};
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

/// A root next to `outside/secret.txt`. The root has `data/a.txt`,
/// `escape` → outside, `leak.txt` → the secret, `inner.txt` → `data/a.txt`
/// and `work/up` → the root.
struct Fixture {
    parent: TempDir,
    ctx: AppContext,
    chroot: Chroot,
}

impl Fixture {
    async fn new() -> Self {
        let parent = TempDir::new().unwrap();
        let root = parent.path().join("root");
        let outside = parent.path().join("outside");
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(root.join("work")).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "secret").unwrap();
        fs::write(root.join("data/a.txt"), "one\ntwo\nthree\n").unwrap();
        fs::write(root.join("database.txt"), "db").unwrap();
        symlink(&outside, root.join("escape")).unwrap();
        symlink(outside.join("secret.txt"), root.join("leak.txt")).unwrap();
        symlink("data/a.txt", root.join("inner.txt")).unwrap();
        symlink("..", root.join("work/up")).unwrap();

        let ctx = AppContext::new(Arc::new(ContextStandard::new(
            "test".to_string(),
            Arc::new(ConfigurationMap::default()),
        )));
        let chroot = Chroot::new(
            &ctx,
            Arc::new(Disk::new(&ctx, &root.to_string_lossy()).unwrap()),
            "/",
            false,
        )
        .await
        .unwrap();
        Self {
            parent,
            ctx,
            chroot,
        }
    }

    fn root(&self) -> PathBuf {
        self.parent.path().join("root")
    }

    fn outside(&self) -> PathBuf {
        self.parent.path().join("outside")
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

    async fn call_in(
        &self,
        chroot: Chroot,
        tool: &(dyn Tool + Send + Sync),
        scopes: &[&str],
        arguments: &[(&str, CallValue)],
    ) -> Result<String, AppError> {
        self.call_with(&mut Fixture::conversation(chroot), tool, scopes, arguments)
            .await
    }

    /// Calls `tool` in `conversation`, so several calls share its values.
    async fn call_with(
        &self,
        conversation: &mut Conversation,
        tool: &(dyn Tool + Send + Sync),
        scopes: &[&str],
        arguments: &[(&str, CallValue)],
    ) -> Result<String, AppError> {
        let name = tool.name(&self.ctx).await.unwrap();
        let call = Call::new(
            name.clone(),
            arguments
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect::<HashMap<_, _>>(),
        );
        let permissions = ToolPermission {
            name,
            scopes: scopes
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
        };
        tool.call(&self.ctx, conversation, permissions, &call).await
    }

    async fn call(
        &self,
        tool: &(dyn Tool + Send + Sync),
        arguments: &[(&str, CallValue)],
    ) -> Result<String, AppError> {
        self.call_in(self.chroot.clone(), tool, &[], arguments)
            .await
    }

    fn assert_untouched(&self) {
        let outside: Vec<String> = fs::read_dir(self.outside())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(outside, ["secret.txt"]);
        assert_eq!(
            fs::read_to_string(self.outside().join("secret.txt")).unwrap(),
            "secret"
        );
    }

    fn assert_no_host_path(&self, error: &AppError) {
        let host = self.parent.path().to_string_lossy().to_string();
        assert!(
            !error.internal_message.contains(&host),
            "{}",
            error.internal_message
        );
    }
}

fn text(value: &str) -> CallValue {
    CallValue::String(value.to_string())
}

/// Writes `content` to `path` like another process, with a modified time
/// distinct from the current one even on coarse file system clocks.
fn change_externally(path: &Path, content: &str) {
    let modified = fs::metadata(path).unwrap().modified().unwrap();
    fs::write(path, content).unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified + Duration::from_secs(1))
        .unwrap();
}

/// Arguments for a `file_write` replacing line 2 of `path` with `content`.
fn edit(path: &str, content: &str) -> [(&'static str, CallValue); 4] {
    [
        ("path", text(path)),
        ("content", text(content)),
        ("start", CallValue::Integer(2)),
        ("length", CallValue::Integer(1)),
    ]
}

#[tokio::test]
async fn mkdir_stays_inside_the_chroot() {
    let fixture = Fixture::new().await;
    for path in ["/../../tmp/escape", "/escape/dir", "/leak.txt/dir"] {
        let error = fixture
            .call(&tools::Mkdir, &[("path", text(path))])
            .await
            .unwrap_err();
        fixture.assert_no_host_path(&error);
    }
    fixture.assert_untouched();

    let created = fixture
        .call(&tools::Mkdir, &[("path", text("/new/nested"))])
        .await
        .unwrap();
    assert_eq!(created, "Created directory '/new/nested'");
    assert!(fixture.root().join("new/nested").is_dir());
}

#[tokio::test]
async fn write_does_not_follow_symlinks_out_of_the_chroot() {
    let fixture = Fixture::new().await;
    for path in [
        "/leak.txt",
        "/escape/secret.txt",
        "/escape/new.txt",
        "/../x.txt",
    ] {
        let error = fixture
            .call(
                &tools::Write,
                &[("path", text(path)), ("content", text("x"))],
            )
            .await
            .unwrap_err();
        fixture.assert_no_host_path(&error);
    }
    fixture.assert_untouched();

    fixture
        .call(
            &tools::Write,
            &[("path", text("/inner.txt")), ("content", text("inner"))],
        )
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "inner"
    );
    assert!(
        fs::symlink_metadata(fixture.root().join("inner.txt"))
            .unwrap()
            .is_symlink()
    );

    let directory = fixture
        .call(
            &tools::Write,
            &[("path", text("/data")), ("content", text("x"))],
        )
        .await;
    assert!(directory.is_err());
}

#[tokio::test]
async fn write_appends_and_replaces_lines() {
    let fixture = Fixture::new().await;
    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    let path = ("path", text("/data/a.txt"));
    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            std::slice::from_ref(&path),
        )
        .await
        .unwrap();
    fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &[
                path.clone(),
                ("content", text("four\n")),
                ("append", CallValue::Boolean(true)),
            ],
        )
        .await
        .unwrap();
    fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &edit("/data/a.txt", "TWO"),
        )
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "one\nTWO\nthree\nfour\n"
    );
}

#[tokio::test]
async fn delete_removes_links_but_not_the_chroot() {
    let fixture = Fixture::new().await;
    let error = fixture
        .call(&tools::Delete, &[("path", text("/"))])
        .await
        .unwrap_err();
    assert!(error.internal_message.contains("can't be deleted"));
    assert!(fixture.root().join("data/a.txt").exists());

    for path in ["/escape", "/leak.txt"] {
        fixture
            .call(&tools::Delete, &[("path", text(path))])
            .await
            .unwrap();
    }
    assert!(!fixture.root().join("escape").exists());
    fixture.assert_untouched();

    fixture
        .call(&tools::Delete, &[("path", text("/work/up/data"))])
        .await
        .unwrap();
    assert!(!fixture.root().join("data").exists());
}

#[tokio::test]
async fn read_rejects_paths_out_of_the_chroot() {
    let fixture = Fixture::new().await;
    for path in [
        "/leak.txt",
        "/escape/secret.txt",
        "/../outside/secret.txt",
        "data/a.txt",
    ] {
        let error = fixture
            .call(&tools::Read, &[("path", text(path))])
            .await
            .unwrap_err();
        fixture.assert_no_host_path(&error);
    }

    let lines = fixture
        .call(
            &tools::Read,
            &[
                ("path", text("/inner.txt")),
                ("start", CallValue::Integer(2)),
                ("end", CallValue::Integer(3)),
            ],
        )
        .await
        .unwrap();
    assert_eq!(lines, "two\nthree");
}

#[tokio::test]
async fn a_sub_chroot_can_not_be_left() {
    let fixture = Fixture::new().await;
    let disk = Arc::new(Disk::new(&fixture.ctx, &fixture.root().to_string_lossy()).unwrap());
    let work = Chroot::new(&fixture.ctx, disk, "/work", false)
        .await
        .unwrap();
    for (tool, path) in [
        (&tools::Read as &(dyn Tool + Send + Sync), "/up/data/a.txt"),
        (&tools::Find, "/up"),
        (&tools::Delete, "/up/data"),
        (&tools::Mkdir, "/up/new"),
    ] {
        let error = fixture
            .call_in(work.clone(), tool, &[], &[("path", text(path))])
            .await
            .unwrap_err();
        assert!(
            error.internal_message.contains("is not accessible"),
            "{}: {}",
            path,
            error.internal_message
        );
    }
    assert!(fixture.root().join("data/a.txt").exists());
    assert!(!fixture.root().join("new").exists());
}

#[tokio::test]
async fn scopes_apply_to_resolved_paths() {
    let fixture = Fixture::new().await;
    let read = async |path: &str| {
        fixture
            .call_in(
                fixture.chroot.clone(),
                &tools::Read,
                &["/data", "/work"],
                &[("path", text(path))],
            )
            .await
    };

    assert!(read("/data/a.txt").await.is_ok());
    assert!(read("/database.txt").await.is_err());
    assert!(read("/inner.txt").await.is_err());
    assert!(read("/work/up/database.txt").await.is_err());
}

#[tokio::test]
async fn find_and_search_list_chroot_paths_without_following_symlinks() {
    let fixture = Fixture::new().await;
    let found = fixture
        .call(&tools::Find, &[("path", text("/"))])
        .await
        .unwrap();
    let paths: Vec<&str> = found
        .lines()
        .skip(1)
        .filter_map(|line| line.split(' ').next())
        .collect();
    assert_eq!(
        paths,
        [
            "/",
            "/data",
            "/data/a.txt",
            "/database.txt",
            "/escape",
            "/inner.txt",
            "/leak.txt",
            "/work",
            "/work/up"
        ]
    );
    assert!(!found.contains(&fixture.parent.path().to_string_lossy().to_string()));

    let symlinks = fixture
        .call(
            &tools::Find,
            &[("path", text("/")), ("file_type", text("symlink"))],
        )
        .await
        .unwrap();
    assert_eq!(symlinks.lines().count(), 5);

    let matches = fixture
        .call(
            &tools::Search,
            &[("path", text("/")), ("pattern", text("^(two|secret|db)$"))],
        )
        .await
        .unwrap();
    assert_eq!(
        matches,
        "Path:Line Match\n/data/a.txt:2 two\n/database.txt:1 db"
    );
}

#[tokio::test]
async fn a_line_edit_of_an_unread_file_fails_and_keeps_it() {
    let fixture = Fixture::new().await;
    let error = fixture
        .call(&tools::Write, &edit("/data/a.txt", "TWO"))
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Validation);
    assert_eq!(error.translation_key, "file_not_read");
    assert_eq!(
        error.internal_message,
        "File '/data/a.txt' was not read or has changed since it was last read. Read it before editing lines."
    );
    assert_eq!(
        fs::read(fixture.root().join("data/a.txt")).unwrap(),
        b"one\ntwo\nthree\n"
    );
}

#[tokio::test]
async fn a_full_or_partial_read_allows_line_edits() {
    let fixture = Fixture::new().await;
    for range in [
        vec![],
        vec![
            ("start", CallValue::Integer(3)),
            ("end", CallValue::Integer(3)),
        ],
    ] {
        let mut conversation = Fixture::conversation(fixture.chroot.clone());
        let mut arguments = vec![("path", text("/data/a.txt"))];
        arguments.extend(range);
        fixture
            .call_with(&mut conversation, &tools::Read, &[], &arguments)
            .await
            .unwrap();
        fixture
            .call_with(
                &mut conversation,
                &tools::Write,
                &[],
                &edit("/data/a.txt", "TWO"),
            )
            .await
            .unwrap();
    }
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "one\nTWO\nthree\n"
    );
}

#[tokio::test]
async fn a_line_edit_after_an_external_change_fails() {
    let fixture = Fixture::new().await;
    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            &[("path", text("/data/a.txt"))],
        )
        .await
        .unwrap();
    change_externally(
        &fixture.root().join("data/a.txt"),
        "zero\none\ntwo\nthree\n",
    );

    let error = fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &edit("/data/a.txt", "TWO"),
        )
        .await
        .unwrap_err();
    assert_eq!(error.translation_key, "file_not_read");
    assert_eq!(
        error.internal_message,
        "File '/data/a.txt' was not read or has changed since it was last read. Read it before editing lines."
    );
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "zero\none\ntwo\nthree\n"
    );
}

#[tokio::test]
async fn own_writes_keep_the_file_current() {
    let fixture = Fixture::new().await;
    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            &[("path", text("/data/a.txt"))],
        )
        .await
        .unwrap();
    for content in ["TWO", "Two\nand a half"] {
        fixture
            .call_with(
                &mut conversation,
                &tools::Write,
                &[],
                &edit("/data/a.txt", content),
            )
            .await
            .unwrap();
    }
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "one\nTwo\nand a half\nthree\n"
    );

    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &[("path", text("/data/a.txt")), ("content", text("a\nb\n"))],
        )
        .await
        .unwrap();
    fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &edit("/data/a.txt", "B"),
        )
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "a\nB\n"
    );
}

#[tokio::test]
async fn an_append_keeps_the_file_current_only_if_it_was() {
    let fixture = Fixture::new().await;
    let append = [
        ("path", text("/data/a.txt")),
        ("content", text("four\n")),
        ("append", CallValue::Boolean(true)),
    ];

    let mut unread = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(&mut unread, &tools::Write, &[], &append)
        .await
        .unwrap();
    let error = fixture
        .call_with(&mut unread, &tools::Write, &[], &edit("/data/a.txt", "TWO"))
        .await
        .unwrap_err();
    assert_eq!(error.translation_key, "file_not_read");

    let mut read = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut read,
            &tools::Read,
            &[],
            &[("path", text("/data/a.txt"))],
        )
        .await
        .unwrap();
    fixture
        .call_with(&mut read, &tools::Write, &[], &append)
        .await
        .unwrap();
    fixture
        .call_with(&mut read, &tools::Write, &[], &edit("/data/a.txt", "TWO"))
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "one\nTWO\nthree\nfour\nfour\n"
    );
}

#[tokio::test]
async fn a_line_edit_of_a_new_file_needs_no_read() {
    let fixture = Fixture::new().await;
    fixture
        .call(
            &tools::Write,
            &[
                ("path", text("/data/new.txt")),
                ("content", text("first\n")),
                ("start", CallValue::Integer(1)),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/new.txt")).unwrap(),
        "first\n"
    );
}

#[tokio::test]
async fn a_symlink_and_its_target_share_the_read_state() {
    let fixture = Fixture::new().await;
    for (read, write) in [("/inner.txt", "/data/a.txt"), ("/data/a.txt", "/inner.txt")] {
        let mut conversation = Fixture::conversation(fixture.chroot.clone());
        fixture
            .call_with(
                &mut conversation,
                &tools::Read,
                &[],
                &[("path", text(read))],
            )
            .await
            .unwrap();
        fixture
            .call_with(&mut conversation, &tools::Write, &[], &edit(write, "TWO"))
            .await
            .unwrap();
    }
    assert_eq!(
        fs::read_to_string(fixture.root().join("data/a.txt")).unwrap(),
        "one\nTWO\nthree\n"
    );
}

#[tokio::test]
async fn a_failed_read_stores_nothing() {
    let fixture = Fixture::new().await;
    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            &[("path", text("/data/missing.txt"))],
        )
        .await
        .unwrap_err();
    assert_eq!(conversation.value("file_read:/data/missing.txt"), None);

    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            &[("path", text("/data/a.txt"))],
        )
        .await
        .unwrap();
    assert!(conversation.value("file_read:/data/a.txt").is_some());
}

#[tokio::test]
async fn the_start_description_asks_for_a_read() {
    let fixture = Fixture::new().await;
    let schema = tools::Write.schema(&fixture.ctx).await.unwrap();
    let schema = serde_json::to_string(&schema).unwrap();
    assert!(schema.contains("Line edits require a current `file_read` of the file"));
}

#[tokio::test]
async fn a_same_size_external_change_is_detected() {
    let fixture = Fixture::new().await;
    let file = fixture.root().join("data/a.txt");
    let mut conversation = Fixture::conversation(fixture.chroot.clone());
    fixture
        .call_with(
            &mut conversation,
            &tools::Read,
            &[],
            &[("path", text("/data/a.txt"))],
        )
        .await
        .unwrap();
    change_externally(&file, "ONE\nTWO\nTHREE\n");

    let error = fixture
        .call_with(
            &mut conversation,
            &tools::Write,
            &[],
            &edit("/data/a.txt", "two"),
        )
        .await
        .unwrap_err();
    assert!(
        error
            .internal_message
            .contains("changed since it was last read")
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), "ONE\nTWO\nTHREE\n");
}

#[tokio::test]
async fn reads_and_writes_of_a_directory_fail() {
    let fixture = Fixture::new().await;
    let path = ("path", text("/data"));
    let content = ("content", text("x"));
    let read = fixture
        .call(&tools::Read, std::slice::from_ref(&path))
        .await;
    let mut results = vec![read];
    for arguments in [
        vec![path.clone(), content.clone()],
        vec![path, content, ("append", CallValue::Boolean(true))],
        edit("/data", "x").to_vec(),
    ] {
        results.push(fixture.call(&tools::Write, &arguments).await);
    }
    for result in results {
        let error = result.unwrap_err();
        assert_eq!(error.translation_key, "file_is_directory");
        assert_eq!(error.internal_message, "Path '/data' is a directory");
    }
    assert!(fixture.root().join("data").is_dir());
}
