// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::tools::models::{SchemaField, SchemaFieldType, Tool};

pub(crate) const REDIRECT_ARGUMENT: &str = "harness_redirect_to_file";
const REDIRECT_DESCRIPTION: &str = "Optional, absolute path within the current chroot. Writes the full output of the tool to this file, replacing it, and returns only a short receipt. Use it for large outputs.";

pub(crate) fn with_redirect(mut tool: Tool) -> Tool {
    tool.schema
        .required
        .retain(|name| name != REDIRECT_ARGUMENT);
    tool.schema.add_property(
        REDIRECT_ARGUMENT,
        false,
        SchemaField::new(SchemaFieldType::String, REDIRECT_DESCRIPTION, None),
    );
    tool
}
