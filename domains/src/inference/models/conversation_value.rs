// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::BTreeMap;

/// A value tools can keep on a conversation between calls. Maps can nest.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversationValue {
    String(String),
    Integer(i64),
    Float(f64),
    Map(BTreeMap<String, ConversationValue>),
}
