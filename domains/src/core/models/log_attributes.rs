// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Serialize, Serializer, ser::SerializeMap};
use serde_json::{Map, Value};

const TRACE_ID: &str = "trace_id";

#[derive(Debug, Clone, PartialEq)]
pub struct LogAttributes {
    pub trace_id: String,
    pub custom: Map<String, Value>,
}

impl LogAttributes {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            custom: Map::new(),
        }
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        let key = key.into();
        if key != TRACE_ID {
            self.custom.insert(key, value.into());
        }
        self
    }
}

impl Serialize for LogAttributes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let custom = self.custom.iter().filter(|(key, _)| *key != TRACE_ID);
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry(TRACE_ID, &self.trace_id)?;
        for (key, value) in custom {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}
