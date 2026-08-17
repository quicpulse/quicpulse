//! Unit tests for JAQ jq-compatible filtering engine and output formatting

use quicpulse::filter::{apply_filter, format_filtered_output};
use serde_json::json;

#[test]
fn test_filter_identity_and_field_access() {
    let data = json!({
        "user": {
            "name": "Alice",
            "age": 30,
            "roles": ["admin", "developer"]
        }
    });

    let id_res = apply_filter(&data, ".").unwrap();
    assert_eq!(id_res.len(), 1);
    assert_eq!(id_res[0], data);

    let name_res = apply_filter(&data, ".user.name").unwrap();
    assert_eq!(name_res, vec![json!("Alice")]);

    let roles_res = apply_filter(&data, ".user.roles[]").unwrap();
    assert_eq!(roles_res, vec![json!("admin"), json!("developer")]);
}

#[test]
fn test_filter_transformations_and_functions() {
    let data = json!([
        {"id": 1, "active": true},
        {"id": 2, "active": false},
        {"id": 3, "active": true}
    ]);

    let mapped = apply_filter(&data, "map(select(.active)) | length").unwrap();
    assert_eq!(mapped, vec![json!(2)]);

    let keys = apply_filter(&json!({"a": 1, "b": 2}), "keys").unwrap();
    assert_eq!(keys, vec![json!(["a", "b"])]);
}

#[test]
fn test_filter_invalid_expression() {
    let data = json!({"foo": "bar"});
    let invalid_res = apply_filter(&data, ".[invalid syntax");
    assert!(invalid_res.is_err());
}

#[test]
fn test_format_filtered_output() {
    let single = vec![json!({"id": 1})];
    let compact = format_filtered_output(&single, false);
    assert_eq!(compact, "{\"id\":1}");

    let pretty = format_filtered_output(&single, true);
    assert!(pretty.contains('\n') && pretty.contains("  \"id\": 1"));

    let empty: Vec<serde_json::Value> = vec![];
    assert_eq!(format_filtered_output(&empty, false), "");
}
