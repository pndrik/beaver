// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelAdapter {
    OpenAI,
    OpenAIResp,
    Anthropic,
    Gemini,
}

macro_rules! models {
    ($($variant:ident => $id:literal, $adapter:ident);* $(;)?) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Model {
            $($variant,)*
        }

        impl Model {
            pub fn from_id(id: &str) -> Option<Self> {
                match id {
                    $($id => Some(Model::$variant),)*
                    _ => None,
                }
            }

            pub fn id(&self) -> &'static str {
                match self {
                    $(Model::$variant => $id,)*
                }
            }

            pub fn adapter(&self) -> ModelAdapter {
                match self {
                    $(Model::$variant => ModelAdapter::$adapter,)*
                }
            }

            pub fn supports_temperature(&self) -> bool {
                !matches!(self, Model::ClaudeFable5 | Model::ClaudeSonnet5)
            }

            pub fn supports_nucleus(&self) -> bool {
                !matches!(self, Model::ClaudeFable5 | Model::ClaudeSonnet5)
            }

            pub fn supports_reasoning_effort(&self) -> bool {
                matches!(
                    self.adapter(),
                    ModelAdapter::OpenAIResp | ModelAdapter::Anthropic | ModelAdapter::Gemini
                )
            }
        }
    };
}

models! {
    GPT5_5           => "gpt-5.5",            OpenAIResp;
    GPT5_5Pro        => "gpt-5.5-pro",        OpenAIResp;
    GPT5_4           => "gpt-5.4",            OpenAIResp;
    GPT5_4Pro        => "gpt-5.4-pro",        OpenAIResp;
    GPT5_4Mini       => "gpt-5.4-mini",       OpenAIResp;
    GPT5_4Nano       => "gpt-5.4-nano",       OpenAIResp;
    GPT5_3Codex      => "gpt-5.3-codex",      OpenAIResp;
    GPT5_3CodexSpark => "gpt-5.3-codex-spark",OpenAIResp;
    GPT5_2           => "gpt-5.2",            OpenAIResp;
    GPT5_2Codex      => "gpt-5.2-codex",      OpenAIResp;
    GPT5_1           => "gpt-5.1",            OpenAIResp;
    GPT5_1Codex      => "gpt-5.1-codex",      OpenAIResp;
    GPT5_1CodexMax   => "gpt-5.1-codex-max",  OpenAIResp;
    GPT5_1CodexMini  => "gpt-5.1-codex-mini", OpenAIResp;
    GPT5             => "gpt-5",              OpenAIResp;
    GPT5Codex        => "gpt-5-codex",        OpenAIResp;
    GPT5Nano         => "gpt-5-nano",         OpenAIResp;

    ClaudeFable5     => "claude-fable-5",     Anthropic;
    ClaudeOpus4_8    => "claude-opus-4-8",    Anthropic;
    ClaudeOpus4_7    => "claude-opus-4-7",    Anthropic;
    ClaudeOpus4_6    => "claude-opus-4-6",    Anthropic;
    ClaudeOpus4_5    => "claude-opus-4-5",    Anthropic;
    ClaudeOpus4_1    => "claude-opus-4-1",    Anthropic;
    ClaudeSonnet5    => "claude-sonnet-5",    Anthropic;
    ClaudeSonnet4_6  => "claude-sonnet-4-6",  Anthropic;
    ClaudeSonnet4_5  => "claude-sonnet-4-5",  Anthropic;
    ClaudeSonnet4    => "claude-sonnet-4",    Anthropic;
    ClaudeHaiku4_5   => "claude-haiku-4-5",   Anthropic;
    ClaudeHaiku3_5   => "claude-3-5-haiku",   Anthropic;

    DeepSeek4Pro     => "deepseek-v4-pro",    OpenAI;
    DeepSeek4Flash   => "deepseek-v4-flash",  OpenAI;

    Gemini3_1Pro     => "gemini-3.1-pro",     Gemini;
    Gemini3_5Flash   => "gemini-3.5-flash",   Gemini;
    Gemini3Flash     => "gemini-3-flash",     Gemini;

    GrokBuild0_1     => "grok-build-0.1",     OpenAI;

    GLM5_2           => "glm-5.2",            OpenAI;
    GLM5_1           => "glm-5.1",            OpenAI;
    GLM5             => "glm-5",              OpenAI;

    KimiK2_7Code     => "kimi-k2.7-code",     OpenAI;
    KimiK2_5         => "kimi-k2.5",          OpenAI;
    KimiK2_6         => "kimi-k2.6",          OpenAI;

    MiniMaxM3        => "minimax-m3",         OpenAI;
    MiniMaxM2_5      => "minimax-m2.5",       OpenAI;
    MiniMaxM2_7      => "minimax-m2.7",       OpenAI;

    Qwen3_7Max       => "qwen3.7-max",        Anthropic;
    Qwen3_7Plus      => "qwen3.7-plus",       Anthropic;
    Qwen3_6Plus      => "qwen3.6-plus",       Anthropic;
    Qwen3_5Plus      => "qwen3.5-plus",       Anthropic;
}

impl Serialize for Model {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Model::from_id(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown model: {s}")))
    }
}
