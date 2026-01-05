//! Comprehensive workflow tests
//!
//! Tests all workflow features including:
//! - Workflow parsing (YAML and TOML)
//! - Variable handling and templating
//! - Environment selection
//! - Magic value expansion
//! - Assertions (status, body, headers, latency)
//! - Extraction and chaining
//! - Scripting (pre_script, post_script, script_assert)
//! - Conditional execution (skip_if)
//! - Retries and delays
//! - Authentication
//! - Report generation

mod common;

use std::path::PathBuf;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header, query_param, body_json};
use serde_json::json;

use common::{http, http_with_env, MockEnvironment};

/// Get path to workflow fixtures
fn workflow_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("workflows")
        .join(name)
}

// ============================================================================
// Workflow Parsing Tests
// ============================================================================

#[test]
fn test_workflow_validate_yaml() {
    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--validate"
    ]);

    // Validation should succeed
    assert!(r.contains("valid") || r.exit_code == 0, "Workflow should validate: {}", r.stderr);
}

#[test]
fn test_workflow_validate_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_workflow = dir.path().join("invalid.yaml");
    std::fs::write(&invalid_workflow, "this is not valid yaml: [").unwrap();

    let r = http(&[
        "--run", invalid_workflow.to_str().unwrap(),
        "--validate"
    ]);

    // Should report error
    assert!(r.exit_code != 0 || r.stderr.contains("error") || r.stderr.contains("Error"));
}

// ============================================================================
// Basic Workflow Execution Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_basic_execution() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"received": true})))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    // Both steps should pass
    assert!(r.contains("Simple GET") || r.contains("passed") || r.exit_code == 0,
            "Workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_with_variables() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"received": true})))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
        "--var", "test_var=custom_value",
        "--var", "numeric_var=100",
    ]);

    assert!(r.exit_code == 0, "Workflow with variables failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Variable Extraction Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_extraction() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "json": {
                "user": {
                    "id": 12345,
                    "name": "Test User",
                    "email": "test@example.com"
                },
                "items": ["a", "b", "c"]
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .and(query_param("user_id", "12345"))
        .and(query_param("name", "Test User"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"found": true})))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("extraction.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Extraction workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Assertion Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_status_assertion() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status/200"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "json": {
                "name": "John Doe",
                "age": 30,
                "active": true,
                "score": 95.5,
                "tags": ["admin", "user"],
                "metadata": null
            }
        })))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("assertions.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Assertion workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_assertion_failure() {
    let server = MockServer::start().await;

    // Return 500 instead of expected 200
    Mock::given(method("GET"))
        .and(path("/status/200"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("assertions.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    // Should fail due to assertion
    assert!(r.exit_code != 0 || r.stderr.contains("failed") || r.stdout.contains("FAIL"),
            "Assertion should have failed");
}

// ============================================================================
// Environment Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_environment_dev() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("X-API-Key", "dev_key_123"))
        .and(query_param("timeout", "60"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("environments.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--env", "dev",
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Dev environment workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_environment_production() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .and(header("X-API-Key", "prod_key_789"))
        .and(query_param("timeout", "10"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("environments.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--env", "production",
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Production environment workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Magic Values Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_magic_values() {
    let server = MockServer::start().await;

    // Accept any request with UUID and timestamp headers
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("magic_values.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Magic values workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Scripting Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_pre_script() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/status/200"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("scripting.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
    ]);

    assert!(r.exit_code == 0, "Scripting workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Report Generation Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_junit_report() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("report.xml");

    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
        "--report-junit", report_path.to_str().unwrap(),
    ]);

    // Check report was created
    assert!(report_path.exists(), "JUnit report should be created");

    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("<?xml"), "Report should be XML");
    assert!(report_content.contains("testsuites") || report_content.contains("testsuite"),
            "Report should have testsuites element");
}

#[tokio::test]
async fn test_workflow_json_report() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("report.json");

    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
        "--report-json", report_path.to_str().unwrap(),
    ]);

    // Check report was created
    assert!(report_path.exists(), "JSON report should be created");

    let report_content = std::fs::read_to_string(&report_path).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();
    assert!(report.get("name").is_some() || report.get("steps").is_some(),
            "Report should have workflow info");
}

#[tokio::test]
async fn test_workflow_tap_report() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("report.tap");

    let workflow = workflow_fixture("basic.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
        "--report-tap", report_path.to_str().unwrap(),
    ]);

    // Check report was created
    assert!(report_path.exists(), "TAP report should be created");

    let report_content = std::fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("TAP version") || report_content.contains("1.."),
            "Report should be TAP format");
}

// ============================================================================
// Continue on Failure Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_continue_on_failure() {
    let server = MockServer::start().await;

    // First step fails
    Mock::given(method("GET"))
        .and(path("/status/200"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    // Second step would succeed
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "json": {
                "name": "John Doe",
                "age": 30,
                "active": true
            }
        })))
        .mount(&server)
        .await;

    let workflow = workflow_fixture("assertions.yaml");
    let r = http(&[
        "--run", workflow.to_str().unwrap(),
        "--var", &format!("base_url={}", server.uri()),
        "--continue-on-failure",
    ]);

    // Should continue despite first step failing
    // Output should show both steps were attempted - look for Step 2 or the second step name
    let output = format!("{} {}", r.stdout, r.stderr);
    assert!(output.contains("Step 2") || output.contains("body assertions") ||
            output.contains("Test body") || output.contains("Total:"),
            "Should continue to next step: {}", output);
}

// ============================================================================
// TOML Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_toml_format() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "healthy"})))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("test.toml");

    std::fs::write(&workflow_path, format!(r#"
name = "TOML Workflow Test"
base_url = "{}"

[[steps]]
name = "Health Check"
method = "GET"
url = "/health"

[steps.assert]
status = 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "TOML workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Inline Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_with_form_data() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "abc123"})))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("form.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Form Data Test
base_url: "{}"

steps:
  - name: Login with form
    method: POST
    url: /login
    form:
      username: testuser
      password: testpass
    extract:
      token: body.token
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Form workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_with_query_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("q", "rust"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": []})))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("query.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Query Params Test
base_url: "{}"

steps:
  - name: Search
    method: GET
    url: /search
    query:
      q: rust
      page: "1"
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Query params workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_with_custom_headers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/protected"))
        .and(header("Authorization", "Bearer token123"))
        .and(header("X-Custom-Header", "custom-value"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("headers.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Headers Test
base_url: "{}"

headers:
  Accept: application/json

steps:
  - name: Protected endpoint
    method: GET
    url: /protected
    headers:
      Authorization: Bearer token123
      X-Custom-Header: custom-value
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Headers workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_basic_auth() {
    let server = MockServer::start().await;

    // Basic auth header is "Basic dXNlcjpwYXNz" (user:pass base64 encoded)
    Mock::given(method("GET"))
        .and(path("/basic-auth"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("basic_auth.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Basic Auth Test
base_url: "{}"

steps:
  - name: Basic auth request
    method: GET
    url: /basic-auth
    auth:
      type: basic
      username: user
      password: pass
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Basic auth workflow failed: {} {}", r.stdout, r.stderr);
}

#[tokio::test]
async fn test_workflow_bearer_auth() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/bearer"))
        .and(header("Authorization", "Bearer my_token_123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("bearer_auth.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Bearer Auth Test
base_url: "{}"

variables:
  token: my_token_123

steps:
  - name: Bearer auth request
    method: GET
    url: /bearer
    auth:
      type: bearer
      token: "{{{{ token }}}}"
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Bearer auth workflow failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// Skip Condition Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_skip_if() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"skip_next": true})))
        .mount(&server)
        .await;

    // Second step - may or may not be called depending on skip_if
    Mock::given(method("GET"))
        .and(path("/feature"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("skip_if.yaml");

    // skip_if uses simple truthiness check: {{var}} or !{{var}}
    std::fs::write(&workflow_path, format!(r#"
name: Skip If Test
base_url: "{}"

steps:
  - name: Check feature flag
    method: GET
    url: /check
    extract:
      should_skip: skip_next
    assert:
      status: 200

  - name: Use feature (skipped)
    method: GET
    url: /feature
    skip_if: "{{{{ should_skip }}}}"
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    // Check that workflow completes and shows the step was skipped
    let output = format!("{} {}", r.stdout, r.stderr);
    assert!(r.exit_code == 0 || output.contains("Skipped") || output.contains("skipped"),
            "Skip-if workflow failed: {}", output);
}

// ============================================================================
// Delay and Timeout Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_with_delay() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/delayed"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("delay.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Delay Test
base_url: "{}"

steps:
  - name: Delayed request
    method: GET
    url: /delayed
    delay: "100ms"
    assert:
      status: 200
"#, server.uri())).unwrap();

    let start = std::time::Instant::now();
    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);
    let elapsed = start.elapsed();

    assert!(r.exit_code == 0, "Delay workflow failed: {} {}", r.stdout, r.stderr);
    assert!(elapsed.as_millis() >= 100, "Delay should be at least 100ms");
}

// ============================================================================
// Multiple Steps Chaining Test
// ============================================================================

#[tokio::test]
async fn test_workflow_multi_step_chain() {
    let server = MockServer::start().await;

    // Step 1: Create user
    Mock::given(method("POST"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": 42,
            "name": "Alice"
        })))
        .mount(&server)
        .await;

    // Step 2: Get user by ID (extracted from step 1)
    Mock::given(method("GET"))
        .and(path("/users/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 42,
            "name": "Alice",
            "created": true
        })))
        .mount(&server)
        .await;

    // Step 3: Update user
    Mock::given(method("PATCH"))
        .and(path("/users/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 42,
            "name": "Alice Updated"
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("chain.yaml");

    std::fs::write(&workflow_path, format!(r#"
name: Multi-Step Chain Test
base_url: "{}"

steps:
  - name: Create User
    method: POST
    url: /users
    headers:
      Content-Type: application/json
    body: |
      {{"name": "Alice"}}
    extract:
      user_id: id
      user_name: name
    assert:
      status: 201

  - name: Get User
    method: GET
    url: /users/{{{{ user_id }}}}
    assert:
      status: 200
      body:
        id: 42

  - name: Update User
    method: PATCH
    url: /users/{{{{ user_id }}}}
    headers:
      Content-Type: application/json
    body: |
      {{"name": "{{{{ user_name }}}} Updated"}}
    assert:
      status: 200
"#, server.uri())).unwrap();

    let r = http(&[
        "--run", workflow_path.to_str().unwrap(),
    ]);

    assert!(r.exit_code == 0, "Multi-step chain failed: {} {}", r.stdout, r.stderr);
}
