// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, Finalize, JsData, JsError, JsObject, JsResult, JsString, Trace, js_error,
};
use boa_runtime::fetch::{Fetcher, request::JsRequest, response::JsResponse};
use std::{cell::RefCell, rc::Rc};

use app_domains::tools::models::{SandboxFetchScope, SandboxScopes};

const MAX_REDIRECTS: usize = 10;

#[derive(Default, Debug, Clone, Finalize, Trace, JsData)]
pub(super) struct FilteredFetcher {
    #[unsafe_ignore_trace]
    scopes: SandboxScopes,
}

impl FilteredFetcher {
    pub fn new(fetch: Vec<SandboxFetchScope>) -> Self {
        Self {
            scopes: SandboxScopes {
                fetch,
                filesystem: vec![],
            },
        }
    }

    pub(super) fn is_allowed_url(&self, uri: &str) -> bool {
        self.scopes.allows_url(uri)
    }

    pub(super) fn is_allowed(&self, uri: &str, method: &str) -> bool {
        self.scopes.allows_fetch(uri, method)
    }
}

impl Fetcher for FilteredFetcher {
    fn resolve_uri(&self, uri: String, _context: &mut Context) -> JsResult<String> {
        if !self.is_allowed_url(&uri) {
            return Err(js_error!(
                ReferenceError: "URL not allowed for fetch: {}",
                uri
            ));
        }

        Ok(uri)
    }

    async fn fetch(
        self: Rc<Self>,
        request: JsRequest,
        _signal: Option<JsObject>,
        _context: &RefCell<&mut Context>,
    ) -> JsResult<JsResponse> {
        let req = request.clone().into_inner();
        let uri = req.uri().to_string();
        let method = req.method().as_str().to_uppercase();

        if !self.is_allowed(&uri, &method) {
            return Err(js_error!(
                ReferenceError: "Request not allowed for fetch: {} {}",
                method,
                uri
            ));
        }

        let fetcher = (*self).clone();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                if !fetcher.is_allowed_url(attempt.url().as_str()) {
                    let message = format!("Redirect not allowed for fetch: {}", attempt.url());
                    return attempt.error(message);
                }
                if attempt.previous().len() >= MAX_REDIRECTS {
                    return attempt.error("Too many redirects");
                }
                attempt.follow()
            }))
            .build()
            .map_err(JsError::from_rust)?;
        let reqwest_req = reqwest::Request::try_from(req).map_err(JsError::from_rust)?;
        let reqwest_res = client
            .execute(reqwest_req)
            .await
            .map_err(JsError::from_rust)?;

        let status = reqwest_res.status();
        let headers = reqwest_res.headers().clone();
        let bytes = reqwest_res.bytes().await.map_err(JsError::from_rust)?;
        let mut builder = http::Response::builder().status(status.as_u16());

        for k in headers.keys() {
            for v in headers.get_all(k) {
                builder = builder.header(k.as_str(), v);
            }
        }

        builder
            .body(bytes.to_vec())
            .map_err(JsError::from_rust)
            .map(|request| JsResponse::basic(JsString::from(uri), request))
    }
}
