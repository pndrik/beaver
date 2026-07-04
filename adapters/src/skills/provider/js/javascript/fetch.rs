// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{Context, Finalize, JsData, JsError, JsResult, JsString, Trace, js_error};
use boa_runtime::fetch::{Fetcher, request::JsRequest, response::JsResponse};
use regex::Regex;
use std::{cell::RefCell, rc::Rc};

#[derive(Default, Debug, Clone, Finalize, Trace, JsData)]
pub(super) struct FilteredFetcher {
    allowed_urls: Vec<String>,
    allowed_methods: Vec<String>,
}

impl FilteredFetcher {
    pub fn new(allowed_urls: Vec<String>, allowed_methods: Vec<String>) -> Self {
        let methods_upper = allowed_methods
            .into_iter()
            .map(|m| m.to_uppercase())
            .collect::<Vec<String>>();

        Self {
            allowed_urls,
            allowed_methods: methods_upper,
        }
    }

    fn is_allowed_url(&self, uri: &str) -> bool {
        if self.allowed_urls.is_empty() {
            return true;
        }

        for url in &self.allowed_urls {
            if url.starts_with("^") {
                let re = match Regex::new(&url) {
                    Ok(re) => re,
                    Err(_) => continue,
                };

                if re.is_match(uri) {
                    return true;
                }
                continue;
            }

            if url == uri {
                return true;
            }
        }

        false
    }

    fn is_allowed_method(&self, method: &str) -> bool {
        if self.allowed_methods.is_empty() {
            return true;
        }

        let method_upper = method.to_uppercase();
        self.allowed_methods.contains(&method_upper)
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
        _context: &RefCell<&mut Context>,
    ) -> JsResult<JsResponse> {
        let req = request.clone().into_inner();
        let uri = req.uri().to_string();
        let method = req.method().as_str().to_uppercase();

        if !self.is_allowed_url(&uri) {
            return Err(js_error!(
                ReferenceError: "URL not allowed for fetch: {} {}",
                method,
                uri
            ));
        }

        if !self.is_allowed_method(&method) {
            return Err(js_error!(
                ReferenceError: "Method not allowed for fetch: {} {}",
                method,
                uri
            ));
        }

        let client = reqwest::Client::new();
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
