// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum LiteralOrRegex {
    Literal(String),
    Regex(Regex),
}

impl LiteralOrRegex {
    pub fn parse(value: &str) -> Result<Self, regex::Error> {
        match value.starts_with('^') {
            true => Regex::new(value).map(Self::Regex),
            false => Ok(Self::Literal(value.to_string())),
        }
    }

    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Literal(literal) => literal == value,
            Self::Regex(regex) => regex.is_match(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Literal(literal) => literal,
            Self::Regex(regex) => regex.as_str(),
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
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
