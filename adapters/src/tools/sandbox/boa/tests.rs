// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{context::ContextStandard, file_system::Disk};
use std::{fs, sync::Arc};

use serde_json::json;
use tempfile::TempDir;

use super::{Boa, fetch::FilteredFetcher, module_loader::ChrootModuleLoader};
use crate::configuration::ConfigurationMap;
use app_domains::{
    core::models::{AppContext, AppError, file::Chroot},
    tools::{
        models::{SandboxLimits, SandboxOutput, SandboxRun, SandboxScopes, SandboxSource},
        traits::CodeSandbox,
    },
};

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

fn scopes(value: serde_json::Value) -> SandboxScopes {
    serde_json::from_value(value).unwrap()
}

fn request(source: SandboxSource) -> SandboxRun {
    SandboxRun {
        source,
        scopes: SandboxScopes::default(),
        limits: None,
    }
}

fn code(code: &str) -> SandboxSource {
    SandboxSource::Code(code.to_string())
}

async fn file_system(directory: &TempDir, path: &str) -> Chroot {
    let ctx = context();
    let disk = Arc::new(Disk::new(&ctx, &directory.path().to_string_lossy()).unwrap());
    Chroot::new(&ctx, disk, path, false).await.unwrap()
}

async fn try_run(chroot: &TempDir, run: SandboxRun) -> Result<SandboxOutput, AppError> {
    Boa.run(&context(), &file_system(chroot, "/").await, run)
        .await
}

async fn run(chroot: &TempDir, run: SandboxRun) -> SandboxOutput {
    try_run(chroot, run).await.unwrap()
}

#[test]
fn fetcher_allows_what_its_scopes_allow() {
    let fetcher = FilteredFetcher::new(
        scopes(json!({ "fetch": [
            { "url": "^https://api\\.example\\.com/", "methods": ["GET"] },
            { "url": "https://exact.example/" }
        ] }))
        .fetch,
    );

    assert!(fetcher.is_allowed("https://api.example.com/v1", "GET"));
    assert!(!fetcher.is_allowed("https://api.example.com/v1", "POST"));
    assert!(fetcher.is_allowed("https://exact.example/", "DELETE"));
    assert!(!fetcher.is_allowed_url("https://exact.example/more"));
    assert!(!fetcher.is_allowed_url("https://api.example.com.evil/"));
    assert!(!FilteredFetcher::new(vec![]).is_allowed_url("https://example.com/"));
}

#[test]
fn module_specifiers_resolve_inside_the_root() {
    let referrer = std::path::Path::new("/lib/a.js");

    assert_eq!(
        ChrootModuleLoader::resolve("/x/y.js", None).unwrap(),
        "/x/y.js"
    );
    assert_eq!(
        ChrootModuleLoader::resolve("./y.js", None).unwrap(),
        "/y.js"
    );
    assert_eq!(
        ChrootModuleLoader::resolve("./b.js", Some(referrer)).unwrap(),
        "/lib/b.js"
    );
    assert_eq!(
        ChrootModuleLoader::resolve("../c.js", Some(referrer)).unwrap(),
        "/c.js"
    );
    assert_eq!(
        ChrootModuleLoader::resolve("/x/../y.js", None).unwrap(),
        "/y.js"
    );

    assert!(ChrootModuleLoader::resolve("lodash", None).is_err());
    assert!(ChrootModuleLoader::resolve("../../c.js", Some(referrer)).is_err());
    assert!(ChrootModuleLoader::resolve("/../c.js", None).is_err());
    assert!(ChrootModuleLoader::resolve("./a.js", Some(std::path::Path::new("etc/a.js"))).is_err());
}

#[tokio::test]
async fn code_returns_console_and_json() {
    let chroot = tempfile::tempdir().unwrap();
    let output = run(
        &chroot,
        request(code(
            "console.log('a'); return await Promise.resolve({ x: [1, 'b'] });",
        )),
    )
    .await;

    assert_eq!(output.console, "a\n");
    assert_eq!(output.result, Ok(Some(json!({ "x": [1, "b"] }))));
    assert_eq!(run(&chroot, request(code(""))).await.result, Ok(None));
    assert_eq!(
        run(&chroot, request(code("return 'text';"))).await.result,
        Ok(Some(json!("text")))
    );
}

#[tokio::test]
async fn errors_of_the_code_are_in_the_result() {
    let chroot = tempfile::tempdir().unwrap();
    let output = run(
        &chroot,
        request(code("console.log('before'); throw new TypeError('nope');")),
    )
    .await;

    assert_eq!(output.console, "before\n");
    assert!(output.result.unwrap_err().contains("TypeError: nope"));
}

#[tokio::test]
async fn files_return_their_default_export() {
    let chroot = tempfile::tempdir().unwrap();
    fs::write(chroot.path().join("lib.js"), "export const two = 2;").unwrap();
    fs::write(
        chroot.path().join("value.js"),
        "import { two } from './lib.js';\nexport default two + 1;",
    )
    .unwrap();
    fs::write(
        chroot.path().join("function.js"),
        "export default async function () { return 'called'; }",
    )
    .unwrap();

    let file = |path: &str| request(SandboxSource::File(path.to_string()));
    assert_eq!(
        run(&chroot, file("/value.js")).await.result,
        Ok(Some(json!(3)))
    );
    assert_eq!(
        run(&chroot, file("/function.js")).await.result,
        Ok(Some(json!("called")))
    );
    assert!(run(&chroot, file("/missing.js")).await.result.is_err());
}

#[tokio::test]
async fn functions_load_from_their_directory_and_access_the_chroot() {
    let directory = tempfile::tempdir().unwrap();
    let chroot = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("lib.js"),
        "export const read = () => fs.readFile('/input.txt');",
    )
    .unwrap();
    fs::write(
        directory.path().join("tool.js"),
        "import { read } from './lib.js';\nasync function main(input, config) { return `${input.name} ${config.greeting} ${read()}`; }\nexport { main };",
    )
    .unwrap();
    fs::write(chroot.path().join("input.txt"), "from chroot").unwrap();

    let function = |name: &str| SandboxRun {
        source: SandboxSource::Function {
            directory: directory.path().to_string_lossy().to_string(),
            path: "/tool.js".to_string(),
            name: name.to_string(),
            arguments: vec![json!({ "name": "Ana" }), json!({ "greeting": "hi" })],
        },
        scopes: scopes(json!({ "filesystem": [{ "path": "/", "operations": ["read"] }] })),
        limits: None,
    };
    assert_eq!(
        run(&chroot, function("main")).await.result,
        Ok(Some(json!("Ana hi from chroot")))
    );
    assert!(run(&chroot, function("missing")).await.result.is_err());
}

#[tokio::test]
async fn fs_and_fetch_exist_only_with_scopes() {
    let chroot = tempfile::tempdir().unwrap();
    let probe = || code("return [typeof fs, typeof fetch];");
    assert_eq!(
        run(&chroot, request(probe())).await.result,
        Ok(Some(json!(["undefined", "undefined"])))
    );

    let mut allowed = request(probe());
    allowed.scopes = scopes(json!({
        "fetch": [{ "url": "^https://allowed\\.example/" }],
        "filesystem": [{ "path": "/", "operations": ["read"] }]
    }));
    assert_eq!(
        run(&chroot, allowed).await.result,
        Ok(Some(json!(["object", "function"])))
    );
}

#[tokio::test]
async fn filesystem_scopes_limit_paths_and_operations() {
    let chroot = tempfile::tempdir().unwrap();
    fs::create_dir_all(chroot.path().join("data")).unwrap();
    fs::create_dir_all(chroot.path().join("out")).unwrap();
    fs::write(chroot.path().join("data/a.txt"), "a").unwrap();
    fs::write(chroot.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink("../secret.txt", chroot.path().join("data/link.txt")).unwrap();

    let scoped = |script: &str| {
        let mut run = request(code(script));
        run.scopes = scopes(json!({ "filesystem": [
            { "path": "/data", "operations": ["read"] },
            { "path": "/out", "operations": ["write"] }
        ] }));
        run
    };

    assert_eq!(
        run(&chroot, scoped(r#"fs.writeFile("/out/b.txt", fs.readFile("/data/a.txt")); return fs.readDir("/data");"#))
            .await
            .result,
        Ok(Some(json!(["a.txt", "link.txt"])))
    );
    assert_eq!(
        fs::read_to_string(chroot.path().join("out/b.txt")).unwrap(),
        "a"
    );

    for script in [
        r#"fs.readFile("/secret.txt");"#,
        r#"fs.readFile("/data/link.txt");"#,
        r#"fs.writeFile("/data/a.txt", "x");"#,
        r#"fs.readFile("/out/b.txt");"#,
        r#"fs.readDir("/");"#,
    ] {
        let error = run(&chroot, scoped(script)).await.result.unwrap_err();
        assert!(error.contains("is not allowed to"), "{script}: {error}");
    }
    let error = run(&chroot, scoped(r#"fs.readFile("/data/../secret.txt");"#))
        .await
        .result
        .unwrap_err();
    assert!(error.contains("must not contain '..'"), "{}", error);
    assert_eq!(
        fs::read_to_string(chroot.path().join("data/a.txt")).unwrap(),
        "a"
    );
}

#[tokio::test]
async fn fetch_rejects_requests_that_match_no_scope() {
    let chroot = tempfile::tempdir().unwrap();
    let mut denied = request(code(r#"await fetch("https://other.example/");"#));
    denied.scopes = scopes(json!({ "fetch": [{ "url": "^https://allowed\\.example/" }] }));

    let error = run(&chroot, denied).await.result.unwrap_err();
    assert!(error.contains("URL not allowed"), "{}", error);
}

#[tokio::test]
async fn limits_stop_runaway_code() {
    let chroot = tempfile::tempdir().unwrap();
    let mut endless = request(code("while (true) {}"));
    endless.limits = Some(SandboxLimits {
        loop_iterations: 1000,
        recursion: 512,
    });

    let error = run(&chroot, endless).await.result.unwrap_err();
    assert!(error.to_lowercase().contains("limit"), "{}", error);
}

#[tokio::test]
async fn a_sub_chroot_confines_files_and_modules() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("sub")).unwrap();
    fs::write(directory.path().join("outer.js"), "export default 'outer';").unwrap();
    fs::write(directory.path().join("sub/inner.txt"), "inner").unwrap();
    std::os::unix::fs::symlink("../outer.js", directory.path().join("sub/up.js")).unwrap();

    let ctx = context();
    let chroot = file_system(&directory, "/sub").await;
    let scoped = |script: &str| {
        let mut run = request(code(script));
        run.scopes = scopes(json!({ "filesystem": [{ "path": "/", "operations": ["read"] }] }));
        run
    };

    let inner = Boa.run(&ctx, &chroot, scoped("return fs.readFile('/inner.txt');"));
    assert_eq!(inner.await.unwrap().result, Ok(Some(json!("inner"))));
    for script in [
        "return fs.readFile('/up.js');",
        "return fs.readFile('/../outer.js');",
    ] {
        let output = Boa.run(&ctx, &chroot, scoped(script)).await.unwrap();
        assert!(output.result.is_err(), "{}", script);
    }
    let file = request(SandboxSource::File("/up.js".to_string()));
    let error = Boa
        .run(&ctx, &chroot, file)
        .await
        .unwrap()
        .result
        .unwrap_err();
    assert!(error.contains("is not accessible"), "{}", error);
}
