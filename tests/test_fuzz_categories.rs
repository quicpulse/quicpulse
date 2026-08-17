//! Unit tests for Security Fuzzing payload categories, risk levels, and custom dictionaries

use quicpulse::fuzz::payloads::{
    create_custom_payloads, generate_payloads, load_custom_payloads_from_file, PayloadCategory,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_payload_category_names_and_all() {
    let all = PayloadCategory::all();
    assert_eq!(all.len(), 10);
    assert_eq!(PayloadCategory::SqlInjection.as_str(), "SQL Injection");
    assert_eq!(PayloadCategory::Xss.as_str(), "XSS");
    assert_eq!(
        PayloadCategory::CommandInjection.as_str(),
        "Command Injection"
    );
    assert_eq!(PayloadCategory::PathTraversal.as_str(), "Path Traversal");
    assert_eq!(PayloadCategory::Boundary.as_str(), "Boundary");
    assert_eq!(PayloadCategory::TypeConfusion.as_str(), "Type Confusion");
    assert_eq!(PayloadCategory::FormatString.as_str(), "Format String");
    assert_eq!(
        PayloadCategory::IntegerOverflow.as_str(),
        "Integer Overflow"
    );
    assert_eq!(PayloadCategory::Unicode.as_str(), "Unicode");
    assert_eq!(PayloadCategory::NoSqlInjection.as_str(), "NoSQL Injection");
    assert_eq!(PayloadCategory::Custom.as_str(), "Custom");
}

#[test]
fn test_generate_payloads_by_category() {
    let sql_payloads = generate_payloads(Some(&[PayloadCategory::SqlInjection]));
    assert!(!sql_payloads.is_empty());
    assert!(sql_payloads
        .iter()
        .all(|p| p.category == PayloadCategory::SqlInjection));

    let xss_payloads = generate_payloads(Some(&[PayloadCategory::Xss]));
    assert!(!xss_payloads.is_empty());
    assert!(xss_payloads
        .iter()
        .all(|p| p.category == PayloadCategory::Xss));
}

#[test]
fn test_custom_payloads_creation_and_file_loading() {
    let cli_payloads = create_custom_payloads(&[
        "' OR 1=1 --".to_string(),
        "<script>alert(1)</script>".to_string(),
    ]);
    assert_eq!(cli_payloads.len(), 2);
    assert_eq!(cli_payloads[0].category, PayloadCategory::Custom);

    let dir = TempDir::new().unwrap();
    let dict_file = dir.path().join("fuzz.txt");
    fs::write(
        &dict_file,
        "payload_one\n# comment line\npayload_two\n\npayload_three\n",
    )
    .unwrap();

    let loaded = load_custom_payloads_from_file(&dict_file).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].category, PayloadCategory::Custom);
}
