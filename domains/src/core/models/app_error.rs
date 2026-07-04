// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::error::Error;

use super::AppContext;

#[derive(Debug)]
pub struct AppError {
    // Internal details (never exposed to clients)
    pub internal_message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
    pub file: &'static str,
    pub line: u64,
    pub module: &'static str,

    // Client-facing
    pub translation_key: &'static str,
    pub kind: ErrorKind,

    // Request context
    pub context: AppContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    Unauthorized,
    Forbidden,
    Validation,
    Conflict,
    Internal,
    ServiceUnavailable,
}

impl ErrorKind {
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::Validation => 422,
            Self::Conflict => 409,
            Self::Internal => 500,
            Self::ServiceUnavailable => 503,
        }
    }
}

#[macro_export]
macro_rules! app_error {
    ($kind:ident, $key:expr, $msg:expr, $ctx:expr) => {
        $crate::core::models::AppError {
            internal_message: $msg.to_string(),
            source: None,
            file: file!(),
            line: line!().into(),
            module: module_path!(),
            translation_key: $key,
            kind: $crate::core::models::ErrorKind::$kind,
            context: $ctx,
        }
    };
}
