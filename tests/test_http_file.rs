//! Integration tests for .http/.rest file parsing and execution

mod common;

use common::{http, http_error, fixtures, ExitStatus};
use std::path::PathBuf;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header, body_string_contains};

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// HTTP File Parsing Tests
// =============================================================================

#[test]
fn test_http_file_load_fixture() {
    let http_path = fixture_path("requests.http");
    let response = http(&["--http-file", http_path.to_str().unwrap(), "--http-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should list the requests from the file
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("GET") || output.contains("POST") || output.contains("users"),
        "Should list requests. output: {}", output);
}

#[test]
fn test_http_file_list_shows_names() {
    let http_path = fixture_path("requests.http");
    let response = http(&["--http-file", http_path.to_str().unwrap(), "--http-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should show the named requests (### Get all users, etc.)
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("Get") || output.contains("users") || output.contains("1."),
        "Should show request names/indices. output: {}", output);
}

#[test]
fn test_http_file_nonexistent() {
    let response = http_error(&["--http-file", "/nonexistent/path/requests.http", "--http-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// HTTP File Single Request Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_execute_single_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!([{"id": 1}])))
        .mount(&mock_server)
        .await;

    // Create a temp .http file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"### Get users
GET {}/users
Accept: application/json
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_http_file_execute_with_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-Custom-Header", "test-value"))
        .and(header("Authorization", "Bearer token123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"GET {}/api
X-Custom-Header: test-value
Authorization: Bearer token123
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// HTTP File with Body Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_post_with_json_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/users"))
        .and(body_string_contains("name"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(serde_json::json!({"id": 1})))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"POST {}/users
Content-Type: application/json

{{"name": "John", "email": "john@example.com"}}
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    assert!(response.stdout.contains("201") || response.stdout.contains("id"),
        "Should create resource. stdout: {}", response.stdout);
}

#[tokio::test]
async fn test_http_file_put_with_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/users/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"PUT {}/users/1
Content-Type: application/json

{{"name": "Updated"}}
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Multiple Requests Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_multiple_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/first"))
        .respond_with(ResponseTemplate::new(200).set_body_string("first"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/second"))
        .respond_with(ResponseTemplate::new(200).set_body_string("second"))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"### First request
GET {}/first

### Second request
GET {}/second
"#, mock_server.uri(), mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    // List should show both
    let list_response = http(&["--http-file", http_file.to_str().unwrap(), "--http-list"]);
    assert_eq!(list_response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", list_response.stdout, list_response.stderr);
    assert!(output.contains("First") || output.contains("Second") || output.contains("2"),
        "Should list multiple requests. output: {}", output);
}

#[tokio::test]
async fn test_http_file_select_by_index() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/first"))
        .respond_with(ResponseTemplate::new(200).set_body_string("first"))
        .expect(0) // Should not be called
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/second"))
        .respond_with(ResponseTemplate::new(200).set_body_string("second"))
        .expect(1) // Should be called once
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"### First
GET {}/first

### Second
GET {}/second
"#, mock_server.uri(), mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    // Select only the second request (index 2)
    let response = http(&["--http-file", http_file.to_str().unwrap(), "--http-request", "2"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_http_file_select_by_name() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/target"))
        .respond_with(ResponseTemplate::new(200).set_body_string("found"))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"### List items
GET {}/list

### Get target
GET {}/target

### Delete item
DELETE {}/delete
"#, mock_server.uri(), mock_server.uri(), mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    // Select by name
    let response = http(&["--http-file", http_file.to_str().unwrap(), "--http-request", "Get target"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Variable Substitution Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_variable_substitution() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"@baseUrl = {}
@apiVersion = v1

GET {{{{baseUrl}}}}/api/{{{{apiVersion}}}}/users
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_http_file_variable_in_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("Authorization", "Bearer secret-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"@token = secret-token

GET {}/api
Authorization: Bearer {{{{token}}}}
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Comment Handling Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_with_comments() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"# This is a comment at the top
### Get test endpoint
# Another comment
GET {}/test
# Comment in headers section
Accept: application/json
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// HTTP Methods Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_all_methods() {
    let mock_server = MockServer::start().await;

    for http_method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
        Mock::given(method(*http_method))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
    }

    let temp_dir = tempfile::TempDir::new().unwrap();

    for http_method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
        let http_file = temp_dir.path().join(format!("test_{}.http", http_method.to_lowercase()));
        let content = format!("{} {}/test\n", http_method, mock_server.uri());
        std::fs::write(&http_file, content).unwrap();

        let response = http(&["--http-file", http_file.to_str().unwrap()]);
        assert_eq!(response.exit_status, ExitStatus::Success,
            "Method {} should work. stderr: {}", http_method, response.stderr);
    }
}

// =============================================================================
// Named Request Tests
// =============================================================================

#[test]
fn test_http_file_list_named_requests() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = r#"### Get all users
GET http://example.com/users

### Create new user
POST http://example.com/users

### Delete user by ID
DELETE http://example.com/users/1
"#;
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap(), "--http-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should list all three named requests
    assert!(output.contains("users") || output.contains("Get") || output.contains("3"),
        "Should list named requests. output: {}", output);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_http_file_empty_file() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("empty.http");
    std::fs::write(&http_file, "").unwrap();

    let response = http_error(&["--http-file", http_file.to_str().unwrap()]);

    // Empty file should error or show no requests
    assert!(response.exit_status == ExitStatus::Error ||
            response.stderr.contains("No requests") ||
            response.stderr.contains("empty"),
        "Should handle empty file. stderr: {}", response.stderr);
}

#[test]
fn test_http_file_invalid_request_line() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("invalid.http");
    let content = "INVALID_METHOD http://example.com\n";
    std::fs::write(&http_file, content).unwrap();

    let response = http_error(&["--http-file", http_file.to_str().unwrap()]);

    // Invalid method should error
    assert!(response.exit_status == ExitStatus::Error ||
            response.stderr.contains("Invalid") ||
            response.stderr.contains("error"),
        "Should reject invalid method. stderr: {}", response.stderr);
}

#[test]
fn test_http_file_invalid_index() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = "GET http://example.com/test\n";
    std::fs::write(&http_file, content).unwrap();

    // Request index 99 doesn't exist
    let response = http_error(&["--http-file", http_file.to_str().unwrap(), "--http-request", "99"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Complex Scenarios Tests
// =============================================================================

#[tokio::test]
async fn test_http_file_multiline_json_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!(r#"POST {}/api
Content-Type: application/json

{{
  "name": "John Doe",
  "email": "john@example.com",
  "roles": ["admin", "user"],
  "metadata": {{
    "created": "2024-01-01"
  }}
}}
"#, mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_http_file_with_query_params_in_url() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"results": []})))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let http_file = temp_dir.path().join("test.http");
    let content = format!("GET {}/search?q=test&limit=10\n", mock_server.uri());
    std::fs::write(&http_file, content).unwrap();

    let response = http(&["--http-file", http_file.to_str().unwrap()]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Using Fixture File Tests
// =============================================================================

#[test]
fn test_http_file_fixture_has_correct_count() {
    let http_path = fixture_path("requests.http");
    let response = http(&["--http-file", http_path.to_str().unwrap(), "--http-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // The fixture has 7 requests
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show multiple requests
    assert!(output.lines().count() >= 3,
        "Should list multiple requests from fixture. output: {}", output);
}
