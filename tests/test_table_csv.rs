//! Unit tests for Table and CSV formatting modules

use quicpulse::table::{format_as_csv, format_as_table};
use serde_json::json;

#[test]
fn test_format_as_table_empty_array() {
    let empty_array = json!([]);
    let table = format_as_table(&empty_array).unwrap();
    assert_eq!(table, "(empty)");
}

#[test]
fn test_format_as_table_valid_objects() {
    let data = json!([
        {"id": 1, "name": "Alice", "active": true},
        {"id": 2, "name": "Bob", "active": false}
    ]);
    let table = format_as_table(&data).unwrap();
    assert!(table.contains("id"));
    assert!(table.contains("name"));
    assert!(table.contains("active"));
    assert!(table.contains("Alice"));
    assert!(table.contains("Bob"));
}

#[test]
fn test_format_as_table_invalid_input() {
    let non_array = json!({"not": "an array"});
    assert!(format_as_table(&non_array).is_err());
}

#[test]
fn test_format_as_csv_valid_objects() {
    let data = json!([
        {"id": 1, "name": "Alice", "city": "New York"},
        {"id": 2, "name": "Bob, Jr.", "city": "London"}
    ]);
    let csv = format_as_csv(&data).unwrap();
    assert!(csv.contains("id") && csv.contains("name") && csv.contains("city"));
    assert!(csv.contains("Alice"));
    assert!(csv.contains("Bob, Jr."));
}

#[test]
fn test_format_as_csv_empty_and_invalid() {
    let empty = json!([]);
    assert!(format_as_csv(&empty).is_ok());

    let non_array = json!("string");
    assert!(format_as_csv(&non_array).is_err());
}
