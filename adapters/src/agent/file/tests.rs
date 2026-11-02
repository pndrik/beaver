// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{fs, path::Path, sync::Arc};

use app_domains::{
    core::models::{AppContext, AppError},
    inference::traits::AgentProvider,
};

use super::File;
use crate::{configuration::ConfigurationMap, context::ContextStandard};

fn context(agents_path: &str) -> AppContext {
    let configuration: ConfigurationMap = [("agents.path", agents_path)].into_iter().collect();
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(configuration),
    )))
}

fn agent_yaml(name: &str, allow: &str) -> String {
    format!(
        r"
version: 1
metadata:
  name: {name}
  display_name: {name}
  description: An agent.
spec:
  inference:
    model: claude-opus-4-8
  tools:
    allow: [{allow}]
  prompt: You help.
"
    )
}

fn write(dir: &Path, file: &str, content: &str) {
    fs::write(dir.join(file), content).unwrap();
}

/// A temp dir with the agents directory `<tmp>/agents`.
fn agents_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let agents = tmp.path().join("agents");
    fs::create_dir(&agents).unwrap();
    (tmp, agents)
}

async fn get(agents: &Path, name: &str) -> Result<String, AppError> {
    let ctx = context(&agents.to_string_lossy());
    File::new(&ctx)
        .get(&ctx, name)
        .await
        .map(|agent| agent.metadata.name)
}

async fn list(agents: &Path) -> Result<Vec<String>, AppError> {
    let ctx = context(&agents.to_string_lossy());
    let mut names: Vec<String> = File::new(&ctx)
        .list(&ctx)
        .await?
        .into_iter()
        .map(|agent| agent.metadata.name)
        .collect();
    names.sort();
    Ok(names)
}

#[tokio::test]
async fn get_loads_a_valid_agent() {
    let (_tmp, agents) = agents_dir();
    write(
        &agents,
        "coder.yaml",
        &agent_yaml("coder", "{ name: ^git_.*$ }"),
    );

    let ctx = context(&agents.to_string_lossy());
    let agent = File::new(&ctx).get(&ctx, "coder").await.unwrap();
    assert_eq!(agent.metadata.name, "coder");
    assert_eq!(agent.spec.prompt, "You help.");
    assert_eq!(agent.spec.tools.allow[0].name, "^git_.*$");
}

#[tokio::test]
async fn get_of_a_missing_agent_is_not_found() {
    let (_tmp, agents) = agents_dir();

    let err = get(&agents, "coder").await.unwrap_err();
    assert_eq!(err.translation_key, "agent_not_found");
    assert_eq!(err.internal_message, "Agent 'coder' not found.");
}

#[tokio::test]
async fn names_that_are_not_plain_file_names_are_not_found() {
    let (tmp, agents) = agents_dir();
    write(tmp.path(), "x.yaml", &agent_yaml("x", ""));
    write(tmp.path(), ".yaml", &agent_yaml("", ""));
    fs::create_dir(agents.join("a")).unwrap();
    write(&agents.join("a"), "b.yaml", &agent_yaml("b", ""));
    write(&agents, ".hidden.yaml", &agent_yaml(".hidden", ""));

    for name in ["", "../x", "a/b", "a\\b", ".hidden", "..", "."] {
        let err = get(&agents, name).await.unwrap_err();
        assert_eq!(err.translation_key, "agent_not_found", "{name}");
    }
}

#[tokio::test]
async fn a_name_mismatch_is_rejected_by_get_and_list() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "coder.yaml", &agent_yaml("Coder", ""));

    let err = get(&agents, "coder").await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert_eq!(
        err.internal_message,
        "Agent name mismatch: expected 'coder', found 'Coder'"
    );

    let err = list(&agents).await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert_eq!(
        err.internal_message,
        "Agent name mismatch: expected 'coder', found 'Coder'"
    );
}

#[tokio::test]
async fn the_mismatch_is_reported_before_validation() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "coder.yaml", &agent_yaml("other", "{ name: '' }"));

    let err = get(&agents, "coder").await.unwrap_err();
    assert_eq!(
        err.internal_message,
        "Agent name mismatch: expected 'coder', found 'other'"
    );
}

#[tokio::test]
async fn list_loads_only_yaml_files() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "a.yaml", &agent_yaml("a", ""));
    write(&agents, "b.yaml", &agent_yaml("b", ""));
    write(&agents, "c.yml", &agent_yaml("c", ""));
    write(&agents, "d.YAML", &agent_yaml("d", ""));
    write(&agents, "e.md", "not an agent");
    fs::create_dir(agents.join("old")).unwrap();
    write(&agents.join("old"), "config.yaml", &agent_yaml("old", ""));
    write(&agents.join("old"), "prompt.md", "You help.");
    fs::create_dir(agents.join("dir.yaml")).unwrap();

    assert_eq!(list(&agents).await.unwrap(), ["a", "b"]);
}

#[tokio::test]
async fn get_does_not_find_other_extensions_or_directories() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "x.yml", &agent_yaml("x", ""));
    write(&agents, "y.YAML", &agent_yaml("y", ""));
    fs::create_dir(agents.join("old")).unwrap();
    write(&agents.join("old"), "config.yaml", &agent_yaml("old", ""));

    write(&agents, "coder.yaml", &agent_yaml("coder", ""));

    for name in ["x", "y", "old", "Coder", "CODER"] {
        let err = get(&agents, name).await.unwrap_err();
        assert_eq!(err.translation_key, "agent_not_found", "{name}");
    }
}

#[tokio::test]
async fn list_fails_if_one_agent_is_invalid() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "a.yaml", &agent_yaml("a", ""));
    write(&agents, "b.yaml", &agent_yaml("b", "{ name: git_.* }"));

    let err = list(&agents).await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert!(
        err.internal_message
            .starts_with("Agent 'b' is invalid: spec.tools.allow[0]: ")
    );
}

#[tokio::test]
async fn get_validates_the_agent() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "a.yaml", &agent_yaml("a", "{ name: ^git_ }"));

    let err = get(&agents, "a").await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert_eq!(
        err.internal_message,
        "Agent 'a' is invalid: spec.tools.allow[0]: name '^git_' must end with '$'"
    );
}

#[tokio::test]
async fn parse_errors_name_the_file() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "a.yaml", "version: 1\nmetadata: {}\n");

    let err = get(&agents, "a").await.unwrap_err();
    assert_eq!(err.translation_key, "configuration_load_failed");
    let path = agents.join("a.yaml");
    assert!(
        err.internal_message
            .starts_with(&format!("Failed to parse agent file {}: ", path.display())),
        "{}",
        err.internal_message
    );
}

#[tokio::test]
async fn a_trailing_slash_in_the_path_is_ignored() {
    let (_tmp, agents) = agents_dir();
    write(&agents, "a.yaml", &agent_yaml("a", ""));
    let path = format!("{}/", agents.display());

    let ctx = context(&path);
    let provider = File::new(&ctx);
    assert_eq!(provider.get(&ctx, "a").await.unwrap().metadata.name, "a");
    assert_eq!(provider.list(&ctx).await.unwrap().len(), 1);
}

#[tokio::test]
async fn the_chart_example_agents_load() {
    let values: yaml_serde::Value =
        yaml_serde::from_str(include_str!("../../../../chart/values.example.yaml")).unwrap();
    let agents = values["agents"].as_mapping().unwrap();
    assert!(!agents.is_empty());

    let (_tmp, dir) = agents_dir();
    for (name, agent) in agents {
        let name = name.as_str().unwrap();
        write(
            &dir,
            &format!("{name}.yaml"),
            &yaml_serde::to_string(agent).unwrap(),
        );
        assert_eq!(get(&dir, name).await.unwrap(), name);
    }
}
