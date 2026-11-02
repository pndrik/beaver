// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A literal, or a regex if it starts with `^`. A regex has to end with `$`
/// and matches only whole values (`^a|b$` is "a or b"). Build it with `parse`.
#[derive(Debug, Clone)]
pub enum LiteralOrRegex {
    Literal(String),
    Regex { pattern: String, regex: Regex },
}

impl LiteralOrRegex {
    pub fn parse(value: &str) -> Result<Self, String> {
        if !value.starts_with('^') {
            return Ok(Self::Literal(value.to_string()));
        }
        if !value.ends_with('$') {
            return Err("must end with '$'".to_string());
        }
        let regex = Regex::new(&format!("^(?:{value})$"))
            .map_err(|err| format!("is not a valid regex: {err}"))?;
        Ok(Self::Regex {
            pattern: value.to_string(),
            regex,
        })
    }

    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Literal(literal) => literal == value,
            Self::Regex { regex, .. } => regex.is_match(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Literal(literal) => literal,
            Self::Regex { pattern, .. } => pattern,
        }
    }
}

impl Serialize for LiteralOrRegex {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LiteralOrRegex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value)
            .map_err(|message| serde::de::Error::custom(format!("'{value}' {message}")))
    }
}
