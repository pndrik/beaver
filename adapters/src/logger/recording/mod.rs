// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Mutex;

use app_domains::core::{models::AppContext, traits::Logger};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone)]
pub struct RecordedLog {
    pub level: RecordedLevel,
    pub trace: String,
    pub message: String,
}

/// Keeps every log message in memory instead of printing it, e.g. to assert on
/// logs in tests.
#[derive(Debug, Default)]
pub struct LoggerRecording {
    logs: Mutex<Vec<RecordedLog>>,
}

impl LoggerRecording {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn logs(&self) -> Vec<RecordedLog> {
        self.logs.lock().unwrap().clone()
    }

    pub fn messages(&self, level: RecordedLevel) -> Vec<String> {
        self.logs
            .lock()
            .unwrap()
            .iter()
            .filter(|log| log.level == level)
            .map(|log| log.message.clone())
            .collect()
    }

    fn record(&self, level: RecordedLevel, ctx: &AppContext, message: &str) {
        self.logs.lock().unwrap().push(RecordedLog {
            level,
            trace: ctx.trace_id.clone(),
            message: message.to_string(),
        });
    }
}

#[async_trait]
impl Logger for LoggerRecording {
    async fn trace(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Trace, ctx, message);
    }
    async fn debug(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Debug, ctx, message);
    }
    async fn info(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Info, ctx, message);
    }
    async fn warn(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Warn, ctx, message);
    }
    async fn error(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Error, ctx, message);
    }
    async fn fatal(&self, ctx: &AppContext, message: &str) {
        self.record(RecordedLevel::Fatal, ctx, message);
    }
}
