// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::Arc,
    thread::{self, JoinHandle},
};

use app_domains::{
    core::models::AppContext,
    inference::{models::Model, traits::InferenceProvider},
};

use super::{CONFIGURATION_APIKEY, CONFIGURATION_ENDPOINT, OpenAI};
use crate::{configuration::ConfigurationMap, context::ContextStandard};

const API_KEY: &str = "sk-test-secret-key";

const MODELS: &str = r#"{ "object": "list", "data": [{ "id": "gpt-5.5" }, { "id": "text-embedding-9" }, { "id": "gpt-6-sol" }] }"#;

/// Serves one HTTP request with `status` and `body`, and returns the request
/// line and headers it received.
fn stub(status: &'static str, body: &'static str) -> (u16, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        for line in BufReader::new(stream.try_clone().unwrap()).lines() {
            let line = line.unwrap();
            if line.is_empty() {
                break;
            }
            request.push(line);
        }
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        request
    });
    (port, handle)
}

fn context(endpoint: &str) -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::from_iter([
            (CONFIGURATION_ENDPOINT, endpoint),
            (CONFIGURATION_APIKEY, API_KEY),
        ])),
    )))
}

#[tokio::test]
async fn supported_models_keeps_only_known_models() {
    let (port, server) = stub("200 OK", MODELS);
    let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

    let models = OpenAI::new(&ctx).supported_models(&ctx).await.unwrap();

    assert_eq!(models, [Model::GPT5_5, Model::GPT6Sol]);
    server.join().unwrap();
}

#[tokio::test]
async fn supports_listed_models_only() {
    for (model, expected) in [(Model::GPT5_5, true), (Model::GPT5_5Pro, false)] {
        let (port, server) = stub("200 OK", MODELS);
        let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

        let supported = OpenAI::new(&ctx).supports_model(&ctx, &model).await;

        assert_eq!(supported.unwrap(), expected, "{}", model.id());
        server.join().unwrap();
    }
}

#[tokio::test]
async fn lists_models_with_a_bearer_token_with_and_without_trailing_slash() {
    for suffix in ["/v1", "/v1/"] {
        let (port, server) = stub("200 OK", MODELS);
        let ctx = context(&format!("http://127.0.0.1:{port}{suffix}"));

        OpenAI::new(&ctx).supported_models(&ctx).await.unwrap();

        let request = server.join().unwrap();
        assert_eq!(request[0], "GET /v1/models HTTP/1.1", "{suffix}");
        assert!(
            request
                .iter()
                .any(|header| header
                    .eq_ignore_ascii_case(&format!("authorization: Bearer {API_KEY}"))),
            "{request:?}"
        );
    }
}

#[tokio::test]
async fn unauthorized_fails_with_the_status_but_not_the_key() {
    let (port, server) = stub(
        "401 Unauthorized",
        r#"{ "error": { "message": "Incorrect API key provided: sk-test-****-key.", "code": "invalid_api_key" } }"#,
    );
    let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

    let err = OpenAI::new(&ctx).supported_models(&ctx).await.unwrap_err();

    assert_eq!(err.translation_key, "inference_failed");
    assert!(
        err.internal_message.contains("401"),
        "{}",
        err.internal_message
    );
    assert!(!err.internal_message.contains(API_KEY));
    server.join().unwrap();
}

#[tokio::test]
async fn non_json_error_body_keeps_the_status() {
    let (port, server) = stub("502 Bad Gateway", "<html>Bad Gateway</html>");
    let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

    let err = OpenAI::new(&ctx).supported_models(&ctx).await.unwrap_err();

    assert_eq!(err.translation_key, "inference_failed");
    assert!(
        err.internal_message.contains("502"),
        "{}",
        err.internal_message
    );
    server.join().unwrap();
}

#[tokio::test]
async fn malformed_listing_fails() {
    for body in [r#"{ "object": "list" }"#, "not json"] {
        let (port, server) = stub("200 OK", body);
        let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

        let err = OpenAI::new(&ctx).supported_models(&ctx).await.unwrap_err();

        assert_eq!(err.translation_key, "inference_failed", "{body}");
        server.join().unwrap();
    }
}

#[tokio::test]
async fn unreachable_endpoint_fails() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let ctx = context(&format!("http://127.0.0.1:{port}/v1"));

    let err = OpenAI::new(&ctx).supported_models(&ctx).await.unwrap_err();

    assert_eq!(err.translation_key, "inference_failed");
    assert!(
        err.internal_message
            .starts_with("Failed to list available models")
    );
}
