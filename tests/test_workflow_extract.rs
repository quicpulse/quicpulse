//! Unit tests for Workflow configuration, environments, and CLI variable overrides

use quicpulse::pipeline::workflow::{apply_cli_variables, apply_environment, load_workflow};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_yaml_workflow_and_environment_override() {
    let dir = TempDir::new().unwrap();
    let wf_path = dir.path().join("workflow.yaml");
    let yaml = r#"
name: E2E Pipeline
description: Test end to end workflow
variables:
  api_url: "https://api.default.com"
  timeout: 5
environments:
  staging:
    api_url: "https://api.staging.internal"
    timeout: 10
  prod:
    api_url: "https://api.prod.com"
steps:
  - name: Step 1
    method: GET
    url: "{{api_url}}/health"
"#;
    fs::write(&wf_path, yaml).unwrap();

    let mut wf = load_workflow(&wf_path).unwrap();
    assert_eq!(wf.name, "E2E Pipeline");
    assert_eq!(
        wf.variables.get("api_url"),
        Some(&json!("https://api.default.com"))
    );

    apply_environment(&mut wf, "staging").unwrap();
    assert_eq!(
        wf.variables.get("api_url"),
        Some(&json!("https://api.staging.internal"))
    );
    assert_eq!(wf.variables.get("timeout"), Some(&json!(10)));
}

#[test]
fn test_apply_cli_variables_override() {
    let dir = TempDir::new().unwrap();
    let wf_path = dir.path().join("workflow.toml");
    let toml_str = r#"
name = "TOML Pipeline"
steps = [
  { name = "Step 1", method = "GET", url = "http://localhost/ping" }
]
"#;
    fs::write(&wf_path, toml_str).unwrap();

    let mut wf = load_workflow(&wf_path).unwrap();
    let cli_vars = vec![
        "user_id=12345".to_string(),
        "is_active=true".to_string(),
        "tag=release_candidate".to_string(),
    ];

    apply_cli_variables(&mut wf, &cli_vars).unwrap();
    assert_eq!(wf.variables.get("user_id"), Some(&json!(12345)));
    assert_eq!(wf.variables.get("is_active"), Some(&json!(true)));
    assert_eq!(wf.variables.get("tag"), Some(&json!("release_candidate")));
}

#[test]
fn test_apply_cli_variables_invalid_format() {
    let dir = TempDir::new().unwrap();
    let wf_path = dir.path().join("wf.yaml");
    fs::write(&wf_path, "name: Test\nsteps:\n  - name: S\n    url: /").unwrap();

    let mut wf = load_workflow(&wf_path).unwrap();
    let invalid_vars = vec!["invalid_without_equal_sign".to_string()];
    assert!(apply_cli_variables(&mut wf, &invalid_vars).is_err());
}
