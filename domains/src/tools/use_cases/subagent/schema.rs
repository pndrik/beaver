// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    tools::models::{Schema, SchemaField, SchemaFieldType},
};

pub(crate) async fn schema(_ctx: &AppContext) -> Result<Schema, AppError> {
    let mut schema = Schema::new("subagent", "");
    schema.add_property("action", true, SchemaField::new(
        SchemaFieldType::String,
        "The action to perform with the subagent. After joining a subagent you can talk to it like to the user.",
        Some(vec![
            "list".to_string(),
            "join".to_string(),
        ])
    ));

    schema.add_property(
        "name",
        true,
        SchemaField::new(
            SchemaFieldType::String,
            "The name of the subagent to interact with.",
            None,
        ),
    );

    schema.add_property(
        "prompt",
        true,
        SchemaField::new(
            SchemaFieldType::String,
            "The prompt to send to the subagent. Only used when action is 'join'.",
            None,
        ),
    );

    Ok(schema)
}
