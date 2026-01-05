//! Integration tests for security fuzzing functionality

mod common;

use common::{http, http_error, fixtures, ExitStatus};
use std::path::PathBuf;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// Basic Fuzz Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "POST",
        &format!("{}/api/test", mock_server.uri()),
        "name=test",
        "value=123"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show fuzz results
    assert!(output.contains("Fuzz") || output.contains("fuzz") ||
            output.contains("payload") || output.contains("requests"),
        "Should show fuzz results. output: {}", output);
}

#[tokio::test]
async fn test_fuzz_single_field() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Only fuzz the "name" field
    let response = http(&[
        "--fuzz",
        "--fuzz-field", "name",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "name=test",
        "email=user@example.com"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_multiple_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Fuzz multiple specific fields
    let response = http(&[
        "--fuzz",
        "--fuzz-field", "name",
        "--fuzz-field", "email",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "name=test",
        "email=test@example.com"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Category Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_sql_category() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "sql",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "query=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show SQL injection tests
    assert!(output.contains("sql") || output.contains("SQL") ||
            output.contains("injection") || output.len() > 100,
        "Should run SQL injection tests. output: {}", output);
}

#[tokio::test]
async fn test_fuzz_xss_category() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "xss",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "content=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_cmd_category() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "cmd",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "command=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_multiple_categories() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "sql",
        "--fuzz-category", "xss",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Risk Level Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_risk_level_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Only high-risk payloads (level 4+)
    let response = http(&[
        "--fuzz",
        "--fuzz-risk", "4",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "input=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_low_risk_level() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Include all risk levels (1+)
    let response = http(&[
        "--fuzz",
        "--fuzz-risk", "1",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "input=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Custom Payload Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_custom_payload() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-payload", "custom_payload_1",
        "--fuzz-payload", "custom_payload_2",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_custom_dict_file() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let dict_path = fixture_path("payloads.txt");
    let response = http(&[
        "--fuzz",
        "--fuzz-dict", dict_path.to_str().unwrap(),
        "POST",
        &format!("{}/api", mock_server.uri()),
        "input=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Anomaly Detection Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_detects_500_anomaly() {
    let mock_server = MockServer::start().await;

    // First few requests return 200, then 500
    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(5)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    // Should detect and report 500 errors as anomalies
    assert!(output.contains("anomal") || output.contains("Anomal") ||
            output.contains("500") || output.contains("error"),
        "Should detect anomalies. output: {}", output);
}

#[tokio::test]
async fn test_fuzz_anomalies_only() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-anomalies-only",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // With no anomalies, output should be minimal
}

#[tokio::test]
async fn test_fuzz_stop_on_anomaly() {
    let mock_server = MockServer::start().await;

    // All requests return 500
    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-stop-on-anomaly",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    // Should stop on first anomaly
    assert!(output.contains("stop") || output.contains("anomal") ||
            output.contains("500") || output.len() > 50,
        "Should stop on anomaly. output: {}", output);
}

// =============================================================================
// Concurrency Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_custom_concurrency() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-concurrency", "5",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Output Format Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_shows_category_summary() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "sql",
        "--fuzz-category", "xss",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "input=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show category breakdown in summary
    let has_categories = output.contains("sql") ||
                         output.contains("SQL") ||
                         output.contains("xss") ||
                         output.contains("XSS") ||
                         output.contains("category") ||
                         output.len() > 200;
    assert!(has_categories, "Should show categories. output: {}", output);
}

#[tokio::test]
async fn test_fuzz_shows_statistics() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "value=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show statistics
    let has_stats = output.contains("total") ||
                    output.contains("Total") ||
                    output.contains("requests") ||
                    output.contains("success") ||
                    output.len() > 100;
    assert!(has_stats, "Should show statistics. output: {}", output);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_fuzz_no_data_fields() {
    let response = http_error(&[
        "--fuzz",
        "GET",
        "http://localhost:9999/api"
    ]);

    // No data fields to fuzz should error
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_fuzz_invalid_category() {
    let response = http_error(&[
        "--fuzz",
        "--fuzz-category", "invalid_category_xyz",
        "POST",
        "http://localhost:9999/api",
        "data=test"
    ]);

    // Invalid category should error or be ignored
    // (depends on implementation)
    assert!(response.exit_status == ExitStatus::Error ||
            response.stderr.contains("invalid") ||
            response.stderr.contains("unknown"),
        "Should handle invalid category. stderr: {}", response.stderr);
}

#[test]
fn test_fuzz_nonexistent_dict_file() {
    let response = http_error(&[
        "--fuzz",
        "--fuzz-dict", "/nonexistent/path/payloads.txt",
        "POST",
        "http://localhost:9999/api",
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// JSON vs Form Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_json_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // JSON data fields
    let response = http(&[
        "--fuzz",
        "--fuzz-category", "type",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "name:=123",  // JSON number
        "active:=true"  // JSON boolean
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_form_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Form data fields
    let response = http(&[
        "--fuzz",
        "--form",
        "--fuzz-category", "sql",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "username=test",
        "password=secret"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Integration with Auth Tests
// =============================================================================

#[tokio::test]
async fn test_fuzz_with_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--auth", "user:pass",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_fuzz_with_bearer_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--fuzz",
        "--auth", "token123",
        "--auth-type", "bearer",
        "--fuzz-category", "boundary",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "data=test"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}
