//! JavaScript Scripting Tests
//!
//! Tests JavaScript scripting support via QuickJS.
//! Tests key modules and script type detection.

mod common;

use common::http;
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a workflow with JavaScript script
fn create_js_script_workflow(server_uri: &str, script: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("js_test.yaml");
    let workflow = format!(
        r#"
name: JavaScript Test
base_url: "{}"

steps:
  - name: Test JavaScript
    method: GET
    url: /test
    script_assert:
      type: javascript
      code: |
{}
"#,
        server_uri,
        script
            .lines()
            .map(|l| format!("        {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    );
    std::fs::write(&workflow_path, workflow).unwrap();
    (dir, workflow_path)
}

/// Helper to create a workflow with JavaScript file
fn create_js_file_workflow(server_uri: &str, script: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();

    // Write JavaScript file
    let script_path = dir.path().join("test.js");
    std::fs::write(&script_path, script).unwrap();

    // Write workflow referencing the .js file
    let workflow_path = dir.path().join("js_file_test.yaml");
    let workflow = format!(
        r#"
name: JavaScript File Test
base_url: "{}"

steps:
  - name: Test JavaScript File
    method: GET
    url: /test
    script_assert:
      file: {}
"#,
        server_uri,
        script_path.to_str().unwrap()
    );
    std::fs::write(&workflow_path, workflow).unwrap();
    (dir, workflow_path)
}

/// Helper to run a JS script test
async fn run_js_test(script: &str) -> (i32, String, String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
        .mount(&server)
        .await;

    let (_dir, workflow_path) = create_js_script_workflow(&server.uri(), script);
    let r = http(&["--run", workflow_path.to_str().unwrap()]);
    (r.exit_code, r.stdout, r.stderr)
}

// ============================================================================
// SCRIPT TYPE DETECTION TESTS
// ============================================================================

#[tokio::test]
async fn test_js_explicit_type_javascript() {
    let (code, stdout, stderr) = run_js_test("true").await;
    assert!(code == 0, "Explicit type:javascript failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_file_extension_detection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok": true}"#))
        .mount(&server)
        .await;

    let (_dir, workflow_path) = create_js_file_workflow(&server.uri(), "response.status === 200");
    let r = http(&["--run", workflow_path.to_str().unwrap()]);
    assert!(r.exit_code == 0, "File extension .js detection failed: {} {}", r.stdout, r.stderr);
}

// ============================================================================
// CRYPTO MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_crypto_sha256() {
    let (code, stdout, stderr) = run_js_test(r#"
crypto.sha256_hex("hello world") === "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
"#).await;
    assert!(code == 0, "crypto.sha256_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_crypto_uuid() {
    let (code, stdout, stderr) = run_js_test(r#"
const uuid = crypto.uuid_v4();
uuid.length === 36 && uuid.includes("-")
"#).await;
    assert!(code == 0, "crypto.uuid_v4 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_crypto_timestamp() {
    let (code, stdout, stderr) = run_js_test(r#"
const ts = crypto.timestamp();
ts > 1700000000
"#).await;
    assert!(code == 0, "crypto.timestamp failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_crypto_random() {
    let (code, stdout, stderr) = run_js_test(r#"
const hex = crypto.random_hex(16);
hex.length === 16
"#).await;
    assert!(code == 0, "crypto.random_hex failed: {} {}", stdout, stderr);
}

// ============================================================================
// ENCODING MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_encoding_base64() {
    let (code, stdout, stderr) = run_js_test(r#"
const encoded = encoding.base64_encode("hello");
const decoded = encoding.base64_decode(encoded);
decoded === "hello"
"#).await;
    assert!(code == 0, "encoding base64 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_encoding_url() {
    let (code, stdout, stderr) = run_js_test(r#"
const encoded = encoding.url_encode("hello world");
encoded === "hello%20world"
"#).await;
    assert!(code == 0, "encoding.url_encode failed: {} {}", stdout, stderr);
}

// ============================================================================
// JSON MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_json_is_valid() {
    let (code, stdout, stderr) = run_js_test(r#"
json.is_valid('{"a": 1}') && !json.is_valid('invalid')
"#).await;
    assert!(code == 0, "json.is_valid failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_json_pretty() {
    let (code, stdout, stderr) = run_js_test(r#"
const pretty = json.pretty('{"a":1}');
pretty.includes("\n")
"#).await;
    assert!(code == 0, "json.pretty failed: {} {}", stdout, stderr);
}

// ============================================================================
// ASSERT MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_assert_eq() {
    let (code, stdout, stderr) = run_js_test(r#"
assert.eq(1, 1);
assert.eq("hello", "hello");
true
"#).await;
    assert!(code == 0, "assert.eq failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_assert_status() {
    let (code, stdout, stderr) = run_js_test(r#"
assert.status_ok(response.status);
true
"#).await;
    assert!(code == 0, "assert.status_ok failed: {} {}", stdout, stderr);
}

// ============================================================================
// STORE MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_store_set_get() {
    let (code, stdout, stderr) = run_js_test(r#"
store.set("test_key", "test_value");
store.get("test_key") === "test_value"
"#).await;
    assert!(code == 0, "store.set/get failed: {} {}", stdout, stderr);
}

// ============================================================================
// HTTP MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_http_status_ok() {
    let (code, stdout, stderr) = run_js_test(r#"
http.is_success(200) && !http.is_success(404)
"#).await;
    assert!(code == 0, "http.is_success failed: {} {}", stdout, stderr);
}

// ============================================================================
// REGEX MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_regex_test() {
    let (code, stdout, stderr) = run_js_test(r#"
regex.test("\\d+", "abc123") && !regex.test("\\d+", "abc")
"#).await;
    assert!(code == 0, "regex.test failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_regex_replace() {
    let (code, stdout, stderr) = run_js_test(r#"
regex.replace("world", "hello world", "JS") === "hello JS"
"#).await;
    assert!(code == 0, "regex.replace failed: {} {}", stdout, stderr);
}

// ============================================================================
// URL MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_url_host() {
    let (code, stdout, stderr) = run_js_test(r#"
url.host("https://example.com/path") === "example.com"
"#).await;
    assert!(code == 0, "url.host failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_url_is_valid() {
    let (code, stdout, stderr) = run_js_test(r#"
url.is_valid("https://example.com") && !url.is_valid("not a url")
"#).await;
    assert!(code == 0, "url.is_valid failed: {} {}", stdout, stderr);
}

// ============================================================================
// DATE MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_date_now() {
    let (code, stdout, stderr) = run_js_test(r#"
const now = date.now();
now.includes("20")  // Year starts with 20
"#).await;
    assert!(code == 0, "date.now failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_date_timestamp() {
    let (code, stdout, stderr) = run_js_test(r#"
date.timestamp() > 1700000000
"#).await;
    assert!(code == 0, "date.timestamp failed: {} {}", stdout, stderr);
}

// ============================================================================
// SYSTEM MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_system_now() {
    let (code, stdout, stderr) = run_js_test(r#"
system.now() > 1700000000
"#).await;
    assert!(code == 0, "system.now failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_system_platform() {
    let (code, stdout, stderr) = run_js_test(r#"
const platform = system.platform();
platform === "macos" || platform === "linux" || platform === "windows"
"#).await;
    assert!(code == 0, "system.platform failed: {} {}", stdout, stderr);
}

// ============================================================================
// FAKER MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_faker_name() {
    let (code, stdout, stderr) = run_js_test(r#"
const name = faker.name();
name.length > 0 && name.includes(" ")
"#).await;
    assert!(code == 0, "faker.name failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_faker_email() {
    let (code, stdout, stderr) = run_js_test(r#"
const email = faker.email();
email.includes("@")
"#).await;
    assert!(code == 0, "faker.email failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_faker_uuid() {
    let (code, stdout, stderr) = run_js_test(r#"
const uuid = faker.uuid();
uuid.length === 36
"#).await;
    assert!(code == 0, "faker.uuid failed: {} {}", stdout, stderr);
}

// ============================================================================
// CONSOLE MODULE TESTS
// ============================================================================

#[tokio::test]
async fn test_js_console_log() {
    let (code, stdout, stderr) = run_js_test(r#"
console.log("test message");
true
"#).await;
    assert!(code == 0, "console.log failed: {} {}", stdout, stderr);
}

// ============================================================================
// RESPONSE CONTEXT TESTS
// ============================================================================

#[tokio::test]
async fn test_js_response_status() {
    let (code, stdout, stderr) = run_js_test(r#"
response.status === 200
"#).await;
    assert!(code == 0, "response.status failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_js_response_body() {
    let (code, stdout, stderr) = run_js_test(r#"
// response.body is already a JS object when the body is valid JSON
response.body.success === true
"#).await;
    assert!(code == 0, "response.body parsing failed: {} {}", stdout, stderr);
}
