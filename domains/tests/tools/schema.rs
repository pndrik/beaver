// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, sync::Arc};

use serde_json::{Value, json};

use app_adapters::{configuration::ConfigurationMap, logger::LoggerRecording};
use app_domains::{
    core::models::{AppContext, AppError},
    tools::models::{Call, CallValue, Schema, SchemaField, SchemaFieldType},
};

fn context() -> AppContext {
    AppContext::new(
        "test".to_string(),
        std::env::temp_dir().to_string_lossy().to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    )
    .unwrap()
}

fn parse(schema: Value) -> Result<Schema, AppError> {
    Schema::from_json_input_schema(&context(), schema.as_object().unwrap().clone())
}

/// A schema with a single optional property `x`.
fn schema_with(property: Value) -> Schema {
    parse(json!({ "type": "object", "properties": { "x": property } })).unwrap()
}

fn call(arguments: Value) -> Call {
    let arguments: HashMap<String, CallValue> = serde_json::from_value(arguments).unwrap();
    Call::new("tool".to_string(), arguments)
}

fn accepts(property: Value, value: Value) -> bool {
    call(json!({ "x": value })).validate_arguments(schema_with(property))
}

#[test]
fn null_arguments_parse() {
    let call = call(json!({ "x": null, "y": [1, null] }));
    assert!(matches!(call.get_argument("x"), Some(CallValue::Null)));
    assert_eq!(call.arguments_as_json_map()["y"], json!([1, null]));
}

#[test]
fn null_is_only_accepted_where_the_schema_allows_it() {
    assert!(accepts(json!({ "type": "null" }), json!(null)));
    assert!(!accepts(json!({ "type": "string" }), json!(null)));
    assert!(!accepts(json!({ "type": "object" }), json!(null)));
    assert!(!accepts(json!({ "type": "null" }), json!("a")));
}

#[test]
fn type_unions_accept_each_listed_type_only() {
    let nullable_string = json!({ "type": ["string", "null"] });
    assert!(accepts(nullable_string.clone(), json!("a")));
    assert!(accepts(nullable_string.clone(), json!(null)));
    assert!(!accepts(nullable_string.clone(), json!(1)));
    assert!(!accepts(nullable_string, json!(["a"])));

    let string_or_integer = json!({ "type": ["string", "integer"] });
    assert!(accepts(string_or_integer.clone(), json!(3)));
    assert!(!accepts(string_or_integer, json!(1.5)));

    assert!(accepts(json!({ "type": ["number", "null"] }), json!(3)));
}

#[test]
fn empty_type_union_accepts_nothing() {
    for value in [json!("a"), json!(1), json!(null), json!({})] {
        assert!(!accepts(json!({ "type": [] }), value));
    }
}

#[test]
fn unknown_types_are_rejected() {
    assert!(parse(json!({ "type": "object", "properties": { "x": { "type": "date" } } })).is_err());
    assert!(
        parse(json!({ "type": "object", "properties": { "x": { "type": ["string", "any"] } } }))
            .is_err()
    );
}

#[test]
fn enums_of_any_json_type_parse_and_are_enforced() {
    let priority = json!({ "type": "integer", "enum": [1, 2, 3] });
    assert!(accepts(priority.clone(), json!(2)));
    assert!(!accepts(priority, json!(4)));

    let ratio = json!({ "type": "number", "enum": [0.5, 1] });
    assert!(accepts(ratio.clone(), json!(0.5)));
    assert!(accepts(ratio.clone(), json!(1.0)));
    assert!(!accepts(ratio, json!(0.75)));

    let flag = json!({ "type": "boolean", "enum": [true] });
    assert!(accepts(flag.clone(), json!(true)));
    assert!(!accepts(flag, json!(false)));

    let mode = json!({ "type": ["string", "null"], "enum": ["fast", null] });
    assert!(accepts(mode.clone(), json!("fast")));
    assert!(accepts(mode.clone(), json!(null)));
    assert!(!accepts(mode, json!("slow")));
}

#[test]
fn string_enums_are_enforced() {
    let mode = json!({ "type": "string", "enum": ["fast", "safe"] });
    assert!(accepts(mode.clone(), json!("safe")));
    assert!(!accepts(mode, json!("unsafe")));
}

#[test]
fn enum_values_must_also_match_the_type() {
    assert!(!accepts(
        json!({ "type": "string", "enum": ["1", 1] }),
        json!(1)
    ));
}

#[test]
fn builder_enums_are_enforced() {
    let mut schema = Schema::new("tool", "");
    schema.add_property(
        "x",
        true,
        SchemaField::new(
            SchemaFieldType::String,
            "",
            Some(vec!["a".to_string(), "b".to_string()]),
        ),
    );
    assert!(call(json!({ "x": "a" })).validate_arguments(schema.clone()));
    assert!(!call(json!({ "x": "c" })).validate_arguments(schema));
}

#[test]
fn required_fields_cannot_be_null_unless_nullable() {
    let schema = parse(json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "required": ["x"]
    }))
    .unwrap();
    assert!(!call(json!({ "x": null })).validate_arguments(schema.clone()));
    assert!(!call(json!({})).validate_arguments(schema));
}

#[test]
fn unions_and_enums_are_sent_to_the_model_unchanged() {
    let property = json!({ "type": ["integer", "null"], "enum": [1, 2, null] });
    let sent = serde_json::to_value(&schema_with(property).properties["x"]).unwrap();
    assert_eq!(sent["type"], json!(["integer", "null"]));
    assert_eq!(sent["enum"], json!([1, 2, null]));
}

#[test]
fn boolean_schemas_are_still_rejected() {
    assert!(parse(json!({ "type": "object", "properties": { "x": true } })).is_err());
    assert!(
        parse(json!({
            "type": "object",
            "properties": { "x": { "type": "array", "items": true } }
        }))
        .is_err()
    );
}
