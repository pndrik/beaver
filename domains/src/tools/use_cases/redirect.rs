// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::tools::models::{Call, CallValue, SchemaField, SchemaFieldType, Tool};

pub(crate) const REDIRECT_ARGUMENT: &str = "redirect_to_file";
const REDIRECT_DESCRIPTION: &str = "Optional, absolute path within the current chroot. Writes the full output of the tool to this file, replacing it, and returns only a short receipt. Use it for large outputs.";

pub(crate) fn defines_redirect(tool: &Tool) -> bool {
    tool.schema.properties.contains_key(REDIRECT_ARGUMENT)
}

pub(crate) fn with_redirect(mut tool: Tool) -> Tool {
    if !defines_redirect(&tool) {
        tool.schema.add_property(
            REDIRECT_ARGUMENT,
            false,
            SchemaField::new(SchemaFieldType::String, REDIRECT_DESCRIPTION, None),
        );
    }
    tool
}

pub(crate) fn take_redirect(call: &Call) -> (Call, Option<String>) {
    let mut call = call.clone();
    let target = match call.arguments.get(REDIRECT_ARGUMENT) {
        Some(CallValue::String(path)) => Some(path.clone()),
        _ => None,
    };
    if target.is_some() {
        call.arguments.remove(REDIRECT_ARGUMENT);
    }
    (call, target)
}

pub(crate) fn receipt(path: &str, output: &str) -> String {
    format!(
        "Output written to '{}' ({} bytes, {} lines)",
        path,
        output.len(),
        output.lines().count()
    )
}
