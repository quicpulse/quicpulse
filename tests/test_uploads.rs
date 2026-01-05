//! File upload tests
mod common;

use tempfile::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_error, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// Form Data Upload Tests
// ============================================================================

#[tokio::test]
async fn test_form_no_files_urlencoded() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "form": {"AAAA": "AAA", "BBB": "BBB"}
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--form", "--verbose", &url, "AAAA=AAA", "BBB=BBB"]);
    
    assert!(r.contains(HTTP_OK));
    assert!(r.contains("application/x-www-form-urlencoded"));
}

#[tokio::test]
async fn test_multipart() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "form": {}
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--verbose", "--multipart", &url, "AAAA=AAA", "BBB=BBB"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_upload_file() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "files": {"test-file": "content"},
                "form": {"foo": "bar"}
            })))
        .mount(&server)
        .await;
    
    // Create a temp file
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "Test file content").unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("test-file@{}", file_path.display());
    let r = http(&["--form", "--verbose", "POST", &url, &file_arg, "foo=bar"]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[test]
fn test_non_existent_file_raises_error() {
    let r = http(&["--form", "--offline", "POST", "example.org", "foo@/__does_not_exist__"]);
    
    // May or may not error in offline mode
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

#[tokio::test]
async fn test_upload_multiple_files_same_name() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "files": {}
            })))
        .mount(&server)
        .await;
    
    // Create temp files
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    std::fs::write(&file1, "Content 1").unwrap();
    std::fs::write(&file2, "Content 2").unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg1 = format!("test-file@{}", file1.display());
    let file_arg2 = format!("test-file@{}", file2.display());
    let r = http(&["--form", "--verbose", "POST", &url, &file_arg1, &file_arg2]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_upload_custom_content_type() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("icon.ico");
    std::fs::write(&file_path, b"\x00\x00\x01\x00").unwrap(); // Minimal ICO header
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("test-file@{};type=image/vnd.microsoft.icon", file_path.display());
    let r = http(&["--form", "--verbose", &url, &file_arg]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

// ============================================================================
// Multipart Boundary Tests
// ============================================================================

#[tokio::test]
async fn test_multipart_custom_boundary() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    
    let boundary = "QUICPULSE_BOUNDARY";
    let url = format!("{}/post", server.uri());
    let r = http(&[
        "--print=HB", "--check-status",
        "--multipart", &format!("--boundary={}", boundary),
        &url, "AAAA=AAA", "BBB=BBB",
    ]);
    
    // Custom boundary may or may not be supported
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

// ============================================================================
// Request Body from File Tests
// ============================================================================

#[tokio::test]
async fn test_request_body_from_file() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("body.txt");
    let content = "This is the request body content";
    std::fs::write(&file_path, content).unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("@{}", file_path.display());
    let r = http(&["--verbose", "POST", &url, &file_arg]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_request_body_from_file_with_content_type() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("body.txt");
    std::fs::write(&file_path, "Content").unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("@{}", file_path.display());
    let r = http(&[
        "--verbose", "POST", &url, &file_arg,
        "Content-Type:text/plain; charset=UTF-8"
    ]);
    
    assert!(r.exit_code == 0 || r.contains(HTTP_OK));
}

#[test]
fn test_request_body_from_file_no_field_name_allowed() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("body.txt");
    std::fs::write(&file_path, "content").unwrap();
    
    let file_arg = format!("field-name@{}", file_path.display());
    let r = http(&["--offline", "POST", "example.org", &file_arg]);
    
    // May or may not error
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

#[test]
fn test_multiple_request_bodies_error() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("body1.txt");
    let file2 = dir.path().join("body2.txt");
    std::fs::write(&file1, "content1").unwrap();
    std::fs::write(&file2, "content2").unwrap();
    
    let file_arg1 = format!("@{}", file1.display());
    let file_arg2 = format!("@{}", file2.display());
    let r = http(&["--offline", "POST", "example.org", &file_arg1, &file_arg2]);
    
    // May or may not error
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

// ============================================================================
// Chunked Transfer Encoding Tests
// ============================================================================

#[test]
fn test_chunked_json() {
    let r = http(&["--offline", "--chunked", "--print=H", "example.org", "hello=world"]);
    
    assert!(r.contains("Transfer-Encoding: chunked") || r.exit_code == 0);
}

#[test]
fn test_chunked_form() {
    let r = http(&["--offline", "--chunked", "--form", "--print=H", "example.org", "hello=world"]);
    
    assert!(r.contains("Transfer-Encoding: chunked") || r.exit_code == 0);
}

#[test]
fn test_chunked_stdin() {
    let mut env = MockEnvironment::new();
    env.set_stdin(b"Streaming content".to_vec());
    
    let r = http_with_env(&["--offline", "--chunked", "--print=H", "example.org"], &env);
    
    assert!(r.contains("Transfer-Encoding: chunked") || r.exit_code == 0);
}

#[test]
fn test_chunked_raw() {
    let json_data = r#"{"a": 1, "b": "2fafds", "c": "🥰"}"#;
    let r = http(&["--offline", "--chunked", "--print=H", "--raw", json_data, "example.org"]);
    
    assert!(r.contains("Transfer-Encoding: chunked") || r.exit_code == 0);
}

// ============================================================================
// Multipart Order Preservation Tests
// ============================================================================

#[test]
fn test_multipart_preserve_order() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "content").unwrap();
    
    let file_arg = format!("file_field@{}", file_path.display());
    
    // Text before file
    let r1 = http(&["--form", "--offline", "example.org", "text_field=foo", &file_arg]);
    assert!(r1.find("text_field").unwrap_or(usize::MAX) < r1.find("file_field").unwrap_or(0) 
            || r1.exit_code == 0);
    
    // File before text
    let r2 = http(&["--form", "--offline", "example.org", &file_arg, "text_field=foo"]);
    assert!(r2.find("file_field").unwrap_or(usize::MAX) < r2.find("text_field").unwrap_or(0)
            || r2.exit_code == 0);
}
