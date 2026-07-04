// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

pub struct Options {
    pub max_tokens: u64,
    pub temperature: f64,
    pub nucleus: f64,
    pub reasoning_effort: ReasoningEffort,
    pub stop_sequences: Vec<String>,
}

impl Options {
    pub fn new(
        max_tokens: u64,
        temperature: f64,
        nucleus: f64,
        reasoning_effort: ReasoningEffort,
        stop_sequences: Vec<String>,
    ) -> Self {
        Self {
            max_tokens,
            temperature,
            nucleus,
            reasoning_effort,
            stop_sequences,
        }
    }

    pub fn default() -> Self {
        Self::new(2048, -1.0, -1.0, ReasoningEffort::Medium, vec![])
    }
}

pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    Max,
}
