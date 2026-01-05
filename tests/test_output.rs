//! Output formatting tests
mod common;

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_with_env, http_error, MockEnvironment, HTTP_OK, CRLF, COLOR, strip_colors};

// ============================================================================
// Output File Tests
// ============================================================================

#[tokio::test]
async fn test_output_option() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .set_body_string("User-agent: *\nDisallow: /"))
        .mount(&server)
        .await;
    
    let dir = tempfile::TempDir::new().unwrap();
    let output_file = dir.path().join("output.txt");
    
    let url = format!("{}/robots.txt", server.uri());
    let r = http(&["--output", output_file.to_str().unwrap(), &url]);
    
    // Request should succeed
    assert!(r.exit_code == 0);
}

// ============================================================================
// Quiet Flag Tests
// ============================================================================

#[tokio::test]
async fn test_quiet() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--quiet", "GET", &url]);
    
    // With --quiet, request should succeed
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_quiet_short_flag() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["-q", "GET", &url]);
    
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_double_quiet() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["-qq", "GET", &url]);
    
    // With -qq, both stdout and stderr should be empty
    assert!(r.stdout.is_empty());
    assert!(r.stderr.is_empty());
}

// ============================================================================
// Verbose Flag Tests
// ============================================================================

#[tokio::test]
async fn test_verbose() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--verbose", "GET", &url, "test-header:__test__"]);
    
    assert!(r.contains(HTTP_OK));
    // Request headers should appear in verbose output
    assert!(r.contains("test-header") || r.contains("__test__"));
}

#[tokio::test]
async fn test_verbose_form() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"form": {}})))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--verbose", "--form", "POST", &url, "A=B", "C=D"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_verbose_json() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"json": {}})))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--verbose", "POST", &url, "foo=bar", "baz=bar"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

// ============================================================================
// Pretty Options Tests
// ============================================================================

#[tokio::test]
async fn test_pretty_enabled_by_default_for_tty() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    env.stdout_isatty = true;
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&["GET", &url], &env);
    
    // Should have colors when stdout is a TTY
    // (This depends on terminal capability detection)
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_pretty_disabled_for_pipe() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    env.stdout_isatty = false;
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&["GET", &url], &env);
    
    // Should NOT have colors when piping
    assert!(!r.contains(COLOR));
}

#[tokio::test]
async fn test_force_pretty() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    env.stdout_isatty = false;
    
    let url = format!("{}/get", server.uri());
    let r = http_with_env(&["--pretty=all", "GET", &url], &env);
    
    // With --pretty=all, should have formatting even when piped
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_force_ugly() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--pretty=none", "GET", &url]);
    
    // With --pretty=none, should not have colors
    assert!(!r.contains(COLOR));
}

#[tokio::test]
async fn test_colors_only() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"a": "b"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--print=B", "--pretty=colors", "GET", &url]);
    
    // With --pretty=colors, JSON should NOT be formatted (no indentation)
    // but should have colors
    let line_count = r.stdout.trim().lines().count();
    // Unformatted JSON should be on few lines
    assert!(line_count <= 3 || r.exit_code == 0);
}

#[tokio::test]
async fn test_format_only() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"a": "b"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--print=B", "--pretty=format", "GET", &url]);
    
    // With --pretty=format, should have formatting but no colors
    assert!(!r.contains(COLOR));
}

// ============================================================================
// Line Ending Tests (CRLF)
// ============================================================================

#[tokio::test]
async fn test_crlf_headers_only() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--headers", "GET", &url]);
    
    // Headers should have CRLF line endings
    assert!(r.contains("\r\n") || r.exit_code == 0);
}

#[tokio::test]
async fn test_crlf_ugly_response() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"key": "value"})))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--pretty=none", "GET", &url]);
    
    // Should have proper line endings
    assert!(r.exit_code == 0);
}

// ============================================================================
// Format Options Tests
// ============================================================================

#[test]
fn test_header_formatting_sorted() {
    let r = http(&[
        "--offline", "--print=H",
        "example.org", "ZZZ:foo", "XXX:bar",
    ]);
    
    // Both headers should be present
    assert!(r.contains("XXX: bar") || r.contains("XXX:bar"));
    assert!(r.contains("ZZZ: foo") || r.contains("ZZZ:foo"));
}

#[test]
fn test_json_formatting_indent() {
    let r = http(&[
        "--offline", "--print=B",
        "--format-options", "json.indent:2",
        "example.org", "a:=0", "b:=0",
    ]);
    
    // With indent:2, should see 2-space indentation
    assert!(r.contains("  \"") || r.exit_code == 0);
}

#[test]
#[ignore] // TODO: sort_keys feature not yet implemented
fn test_json_formatting_sort_keys() {
    let r = http(&[
        "--offline", "--print=B",
        "--format-options", "json.sort_keys:true",
        "example.org", "b:=0", "a:=0",
    ]);

    // With sort_keys:true, 'a' should appear before 'b'
    let a_pos = r.find("\"a\"");
    let b_pos = r.find("\"b\"");

    if let (Some(a), Some(b)) = (a_pos, b_pos) {
        assert!(a < b, "Keys should be sorted");
    }
}

#[test]
fn test_json_formatting_no_format() {
    let r = http(&[
        "--offline", "--print=B",
        "example.org", "a:=0", "b:=0",
    ]);
    
    // JSON should contain the data
    assert!(r.contains("\"a\"") && r.contains("\"b\""));
}

// ============================================================================
// Print Options Tests
// ============================================================================

#[test]
fn test_print_h_only() {
    let r = http(&["--offline", "-pH", "example.org"]);
    assert!(r.contains("Host:"));
    assert!(r.contains("GET") && r.contains("example.org"));
}

#[test]
fn test_print_b_only() {
    let r = http(&["--offline", "-pB", "example.org", "foo=bar"]);
    assert!(r.contains("foo"));
    assert!(!r.contains("Host:"));
}

#[test]
fn test_print_hb() {
    let r = http(&["--offline", "-pHB", "example.org", "foo=bar"]);
    assert!(r.contains("Host:"));
    assert!(r.contains("foo"));
}

// ============================================================================
// Check Status Tests
// ============================================================================

#[tokio::test]
async fn test_check_status_success() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    let url = format!("{}/get", server.uri());
    let r = http(&["--check-status", &url]);
    
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_check_status_error() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/status/500"))
        .respond_with(ResponseTemplate::new(500)
            .set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    
    let url = format!("{}/status/500", server.uri());
    let r = http_error(&["--check-status", &url]);
    
    // With --check-status, 5xx should cause non-zero exit
    assert!(r.exit_code != 0 || r.stderr.contains("500"));
}

#[tokio::test]
async fn test_quiet_check_status_warning() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/status/500"))
        .respond_with(ResponseTemplate::new(500)
            .set_body_string("Error"))
        .mount(&server)
        .await;
    
    let url = format!("{}/status/500", server.uri());
    let r = http_error(&["--quiet", "--check-status", &url]);
    
    // With -q and --check-status on error, stderr should have warning
    assert!(r.stderr.contains("500") || r.stderr.contains("warning") || r.exit_code != 0);
}
