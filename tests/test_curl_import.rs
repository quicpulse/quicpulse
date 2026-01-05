//! Integration tests for curl command import and generation functionality

mod common;

use common::{http, http_error, ExitStatus};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header, body_string_contains};

// =============================================================================
// Curl Import - Basic GET Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_simple_get() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello"))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("200") || response.stdout.contains("Hello"),
        "Should get successful response. stdout: {}", response.stdout);
}

#[tokio::test]
async fn test_curl_import_with_url_only() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!([{"id": 1}])))
        .mount(&mock_server)
        .await;

    // Just URL, no "curl" prefix
    let response = http(&["--import-curl", &format!("curl {}/api/users", mock_server.uri())]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - POST with Data Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_post_json() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/create"))
        .and(body_string_contains("name"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(serde_json::json!({"id": 1, "name": "Test"})))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        r#"curl -X POST -H 'Content-Type: application/json' -d '{{"name":"Test"}}' {}/api/create"#,
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("201") || response.stdout.contains("id"),
        "Should create resource. stdout: {}", response.stdout);
}

#[tokio::test]
async fn test_curl_import_post_form_data() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -X POST -d 'username=admin&password=secret' {}/login",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_data_flag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/submit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // -d flag should imply POST method
    let curl_cmd = format!("curl -d 'key=value' {}/submit", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Headers Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_single_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-Custom-Header", "custom-value"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -H 'X-Custom-Header: custom-value' {}/api",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_multiple_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-Header-One", "value1"))
        .and(header("X-Header-Two", "value2"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -H 'X-Header-One: value1' -H 'X-Header-Two: value2' {}/api",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_accept_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_string("text"))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -H 'Accept: text/plain' {}/api",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Authentication Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_basic_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/secure"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz")) // base64(user:pass)
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -u user:pass {}/secure", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_bearer_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/protected"))
        .and(header("Authorization", "Bearer test-token-123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -H 'Authorization: Bearer test-token-123' {}/protected",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - SSL/TLS Options Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_insecure_flag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // -k flag should be accepted
    let curl_cmd = format!("curl -k {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_insecure_long_flag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl --insecure {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Redirect Options Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_follow_redirects() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/redirect"))
        .respond_with(ResponseTemplate::new(302)
            .append_header("Location", "/final"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Final"))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -L {}/redirect", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("Final") || response.stdout.contains("200"),
        "Should follow redirect. stdout: {}", response.stdout);
}

#[tokio::test]
async fn test_curl_import_location_flag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(301)
            .append_header("Location", "/end"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/end"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl --location {}/start", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Combined Flags Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_combined_ssl_flags() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // -sSL = silent, show errors, follow redirects
    let curl_cmd = format!("curl -sSL {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_combined_kvs_flags() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // -kv = insecure + verbose
    let curl_cmd = format!("curl -kv {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Timeout Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_timeout() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_millis(100)))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -m 5 {}/slow", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_max_time_flag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl --max-time 10 {}/test", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - HTTP Methods Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_put_method() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/resource/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -X PUT -d 'data' {}/resource/1", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_delete_method() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/resource/1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -X DELETE {}/resource/1", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_patch_method() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/resource/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -X PATCH -d 'patch' {}/resource/1", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_head_method() {
    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200)
            .append_header("X-Custom", "value"))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!("curl -I {}/resource", mock_server.uri());
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Generation Tests (--curl flag)
// =============================================================================

#[tokio::test]
async fn test_curl_generate_simple_get() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let response = http(&["--curl", "GET", &format!("{}/test", mock_server.uri())]);

    // Should output curl command, not make request
    assert!(response.stdout.contains("curl") || response.stderr.contains("curl"),
        "Should generate curl command. stdout: {}, stderr: {}", response.stdout, response.stderr);
}

#[tokio::test]
async fn test_curl_generate_with_headers() {
    let mock_server = MockServer::start().await;

    let response = http(&[
        "--curl",
        "GET",
        &format!("{}/test", mock_server.uri()),
        "X-Custom:value"
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    // Should include the header in curl output
    assert!(output.contains("-H") || output.contains("header"),
        "Should include header flag. output: {}", output);
}

#[tokio::test]
async fn test_curl_generate_post_with_data() {
    let mock_server = MockServer::start().await;

    let response = http(&[
        "--curl",
        "POST",
        &format!("{}/api", mock_server.uri()),
        "name=John"
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    // Should include POST method and data
    assert!(output.contains("POST") || output.contains("-X"),
        "Should include method. output: {}", output);
}

#[tokio::test]
async fn test_curl_generate_with_auth() {
    let mock_server = MockServer::start().await;

    let response = http(&[
        "--curl",
        "--auth", "user:pass",
        "GET",
        &format!("{}/secure", mock_server.uri())
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    // Should include -u flag
    assert!(output.contains("-u") || output.contains("Authorization"),
        "Should include auth. output: {}", output);
}

// =============================================================================
// Curl Import - Error Handling Tests
// =============================================================================

#[test]
fn test_curl_import_invalid_url() {
    let response = http_error(&["--import-curl", "curl not-a-valid-url"]);

    // Should fail with invalid URL
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_curl_import_empty_command() {
    let response = http_error(&["--import-curl", ""]);

    // Empty command should error
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_curl_import_unterminated_quote() {
    let response = http_error(&["--import-curl", "curl -H 'Content-Type: application/json https://example.com"]);

    // Unterminated quote should error
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Curl Import - Complex Commands Tests
// =============================================================================

#[tokio::test]
async fn test_curl_import_complex_command() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/create"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    // Complex curl command with multiple options
    let curl_cmd = format!(
        r#"curl -X POST -H 'Content-Type: application/json' -H 'Accept: application/json' -d '{{"name":"Test","value":123}}' -L -k {}/api/create"#,
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_with_user_agent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("User-Agent", "MyApp/1.0"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -A 'MyApp/1.0' {}/api",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_curl_import_with_referer() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/page"))
        .and(header("Referer", "https://google.com"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let curl_cmd = format!(
        "curl -e 'https://google.com' {}/page",
        mock_server.uri()
    );
    let response = http(&["--import-curl", &curl_cmd]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Import - Proxy Tests
// =============================================================================

#[test]
fn test_curl_import_proxy_option() {
    // Just test parsing - don't actually connect via proxy
    let response = http(&["--import-curl", "curl -x http://proxy:8080 http://nonexistent.example.com/test", "--offline"]);

    // Should parse successfully (offline means no actual request)
    // Proxy option should be recognized from the curl command
    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Curl Roundtrip Tests
// =============================================================================

#[test]
fn test_curl_roundtrip_simple() {
    // Test that --curl generates a valid curl command
    // Note: --curl just generates the command without making a request
    let gen_response = http(&[
        "--curl",
        "--offline",  // Don't try to connect
        "GET",
        "http://example.com/roundtrip"
    ]);

    // The generated command should contain curl
    let output = format!("{}{}", gen_response.stdout, gen_response.stderr);
    assert!(output.contains("curl"), "Should generate curl command. output: {}", output);
}
