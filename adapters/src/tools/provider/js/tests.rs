// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, fs, sync::Arc};

use serde_json::json;

use super::JavascriptProvider;
use crate::{configuration::ConfigurationMap, logger::LoggerRecording, tools::sandbox::Boa};
use app_domains::{
    core::models::AppContext,
    inference::models::{
        Agent, AgentCacheLevel, AgentInference, Compression, Conversation, Message, Model, Options,
    },
    tools::{
        models::{Call, CallValue, ToolPermission},
        traits::ToolsProvider,
    },
};

const PACKAGE: &str = r#"{
  "name": "greeter",
  "version": "0.0.1",
  "config": { "beaver": { "tools": {
    "greet": {
      "main": "greet.js",
      "description": "Greets.",
      "scopes": { "filesystem": [{ "path": "/in", "operations": ["read"] }] },
      "parameters": { "type": "object", "properties": { "name": { "type": "string" } } }
    }
  } } }
}"#;

#[tokio::test]
async fn tools_run_in_the_sandbox() {
    let tools = tempfile::tempdir().unwrap();
    let package = tools.path().join("greeter");
    fs::create_dir_all(package.join("lib")).unwrap();
    fs::write(package.join("package.json"), PACKAGE).unwrap();
    fs::write(
        package.join("lib/shared.js"),
        "export const greeting = (config) => config.greeting;",
    )
    .unwrap();
    fs::write(
        package.join("greet.js"),
        r#"import { greeting } from "./lib/shared.js";
async function main(input, config, permissions) {
  console.log("greeting");
  return `${greeting(config)} ${input.name} (${permissions.scopes.join(",")}, ${typeof fetch}, ${fs.readFile("/in/name.txt")})`;
}
export { main };"#,
    )
    .unwrap();

    let ctx = AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::from_iter([
            (
                "tools.providers.javascript.path",
                json!(tools.path().to_string_lossy()),
            ),
            (
                "tools.configuration.js_greeter",
                json!({ "greeting": "Hi" }),
            ),
        ])),
        Arc::new(LoggerRecording::new()),
    );
    let mut provider = JavascriptProvider::new(&ctx, Arc::new(Boa::new()));
    provider.reload(&ctx).await.unwrap();

    let agent = Agent::new(
        "agent".to_string(),
        "agent".to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    let chroot = tempfile::tempdir().unwrap();
    fs::create_dir(chroot.path().join("in")).unwrap();
    fs::write(chroot.path().join("in/name.txt"), "chroot").unwrap();
    let mut conversation = Conversation::new(
        &ctx,
        Message::system(String::new(), &agent),
        agent,
        vec![],
        &chroot.path().to_string_lossy(),
    )
    .unwrap();

    let result = provider
        .execute(
            &ctx,
            &mut conversation,
            ToolPermission {
                name: "js_greet".to_string(),
                scopes: vec!["a".to_string()],
            },
            &Call::new(
                "js_greet".to_string(),
                HashMap::from([("name".to_string(), CallValue::String("Ana".to_string()))]),
            ),
        )
        .await
        .unwrap();

    assert_eq!(result, "Hi Ana (a, undefined, chroot)");
}

#[tokio::test]
async fn example_tools_load() {
    use app_domains::tools::{
        models::{SandboxRun, SandboxScopes, SandboxSource},
        traits::CodeSandbox,
    };

    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/tools");
    let ctx = AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::from_iter([(
            "tools.providers.javascript.path",
            json!(examples.to_string_lossy()),
        )])),
        Arc::new(LoggerRecording::new()),
    );

    let mut provider = JavascriptProvider::new(&ctx, Arc::new(Boa::new()));
    provider.reload(&ctx).await.unwrap();
    assert_eq!(provider.tools.len(), 15);
    assert!(
        provider
            .tools
            .values()
            .all(|tool| !tool.scopes.fetch.is_empty())
    );

    let chroot = tempfile::tempdir().unwrap();
    for (name, tool) in &provider.tools {
        let run = SandboxRun {
            source: SandboxSource::Function {
                directory: tool.directory.clone(),
                path: format!("/{}", tool.main),
                name: "main".to_string(),
                arguments: vec![],
            },
            scopes: SandboxScopes::default(),
            limits: None,
        };
        let output = Boa::new()
            .run(&ctx, &chroot.path().to_string_lossy(), run)
            .await
            .unwrap();
        // Without arguments `main` fails on its input, but only after loading.
        if let Err(e) = &output.result {
            assert!(!e.contains("Cannot import"), "{}: {}", name, e);
            assert!(!e.contains("SyntaxError"), "{}: {}", name, e);
            assert!(!e.contains("not a function"), "{}: {}", name, e);
        }
    }
}
