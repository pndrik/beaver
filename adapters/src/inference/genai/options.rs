// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::ChatOptions;

use super::GenAi;
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::{Model, ModelAdapter, Options, ReasoningEffort},
};

impl GenAi {
    pub(super) fn get_options(
        &self,
        _ctx: &AppContext,
        model: &Model,
        options: &Options,
    ) -> Result<ChatOptions, AppError> {
        let mut o = ChatOptions::default()
            .with_normalize_reasoning_content(true)
            .with_max_tokens(options.max_tokens.min(u32::MAX as u64) as u32);

        if model.supports_temperature() && options.temperature >= 0.0 {
            o = o.with_temperature(options.temperature);
        }
        if model.supports_nucleus() && options.nucleus >= 0.0 {
            o = o.with_top_p(options.nucleus);
        }

        if options.stop_sequences.len() > 0 && model.adapter() != ModelAdapter::OpenAIResp {
            o = o.with_stop_sequences(options.stop_sequences.clone());
        }

        if model.supports_reasoning_effort() {
            o = o.with_reasoning_effort(match options.reasoning_effort {
                ReasoningEffort::Low => genai::chat::ReasoningEffort::Low,
                ReasoningEffort::Medium => genai::chat::ReasoningEffort::Medium,
                ReasoningEffort::High => genai::chat::ReasoningEffort::High,
                ReasoningEffort::Max => {
                    if model.adapter() == ModelAdapter::Anthropic
                        || model.adapter() == ModelAdapter::Gemini
                    {
                        genai::chat::ReasoningEffort::Max
                    } else {
                        genai::chat::ReasoningEffort::XHigh
                    }
                }
            });
        }

        Ok(o)
    }
}
