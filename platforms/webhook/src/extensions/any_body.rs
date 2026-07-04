// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::{
    Json,
    extract::{FromRequest, Request},
    http::{StatusCode, header::CONTENT_TYPE},
};
use serde_json::Value;

pub struct AnyBody(pub Value);

impl<S> FromRequest<S> for AnyBody
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(';').next().unwrap_or("").trim())
            .unwrap_or("");

        let value = match content_type {
            "application/json" => {
                Json::<Value>::from_request(req, state)
                    .await
                    .map_err(bad_request)?
                    .0
            }
            "application/x-www-form-urlencoded" => {
                let bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                    .await
                    .map_err(bad_request)?;
                serde_qs::from_bytes(&bytes).map_err(bad_request)?
            }
            _ => Value::Null,
        };

        Ok(AnyBody(value))
    }
}

fn bad_request(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}
