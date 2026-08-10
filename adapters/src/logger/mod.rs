// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Serialize;
use std::fmt::Debug;

use app_domains::core::{
    models::{AppContext, AppError},
    traits::Logger,
};

const LOG_LEVEL_CONFIGURATION_KEY: &str = "logger.level";

#[derive(Serialize)]
struct LogMessage {
    timestamp: String,
    level: LogLevel,
    trace: String,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn from_str(s: &str) -> Self {
        match s {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "fatal" => LogLevel::Fatal,
            _ => LogLevel::Info, // Default to Info if unrecognized
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggerAdapter {
    level: LogLevel,
}

impl LoggerAdapter {
    pub fn new() -> Self {
        LoggerAdapter {
            level: LogLevel::Info,
        }
    }

    pub async fn refresh_level(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let level_str = ctx
            .configuration
            .get_string(ctx, LOG_LEVEL_CONFIGURATION_KEY)
            .await?;

        self.level = LogLevel::from_str(&level_str);

        Ok(())
    }

    fn log(&self, level: LogLevel, ctx: &AppContext, message: &str) {
        let message = LogMessage {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            trace: ctx.trace_id.clone(),
            message: message.to_string(),
        };

        println!("{}", serde_json::to_string(&message).unwrap());
    }
}

#[async_trait]
impl Logger for LoggerAdapter {
    async fn trace(&self, ctx: &AppContext, message: &str) {
        if self.level < LogLevel::Trace {
            return;
        }

        self.log(LogLevel::Trace, ctx, message);
    }
    async fn debug(&self, ctx: &AppContext, message: &str) {
        if self.level < LogLevel::Debug {
            return;
        }

        self.log(LogLevel::Debug, ctx, message);
    }
    async fn info(&self, ctx: &AppContext, message: &str) {
        if self.level < LogLevel::Info {
            return;
        }

        self.log(LogLevel::Info, ctx, message);
    }
    async fn warn(&self, ctx: &AppContext, message: &str) {
        if self.level < LogLevel::Warn {
            return;
        }

        self.log(LogLevel::Warn, ctx, message);
    }
    async fn error(&self, ctx: &AppContext, message: &str) {
        if self.level < LogLevel::Error {
            return;
        }

        self.log(LogLevel::Error, ctx, message);
    }
    async fn fatal(&self, ctx: &AppContext, message: &str) {
        self.log(LogLevel::Fatal, ctx, message);
    }
}
