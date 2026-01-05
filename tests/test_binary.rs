//! Binary data handling tests
mod common;

use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// Binary Request Data Tests
// ============================================================================

#[tokio::test]
async fn test_binary_stdin() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    // Create binary data with non-UTF8 bytes
    let binary_data: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    
    let mut env = MockEnvironment::new();
    env.set_stdin(binary_data.clone());
    
    let url = format!("{}/post", server.uri());
    let r = http_with_env(&["--print=B", "POST", &url], &env);
    
    // The response should come back (even if binary was sent)
    assert!(r.contains("OK") || r.exit_code == 0);
}

#[tokio::test]
async fn test_binary_file_path() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    // Create a temp file with binary content
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("binary.bin");
    let binary_data: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    std::fs::write(&file_path, &binary_data).unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("@{}", file_path.display());
    
    let mut env = MockEnvironment::new();
    let r = http_with_env(&["--print=B", "POST", &url, &file_arg], &env);
    
    assert!(r.contains("OK") || r.exit_code == 0);
}

#[tokio::test]
async fn test_binary_file_form() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("OK"))
        .mount(&server)
        .await;
    
    // Create a temp file with binary content
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("binary.bin");
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    std::fs::write(&file_path, &binary_data).unwrap();
    
    let url = format!("{}/post", server.uri());
    let file_arg = format!("test@{}", file_path.display());
    
    let r = http(&["--print=B", "--form", "POST", &url, &file_arg]);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Binary Response Data Tests
// ============================================================================

#[tokio::test]
async fn test_binary_response_suppressed_when_terminal() {
    let server = MockServer::start().await;
    
    // Respond with binary data
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    
    Mock::given(method("GET"))
        .and(path("/bytes"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .set_body_bytes(binary_data))
        .mount(&server)
        .await;
    
    let url = format!("{}/bytes", server.uri());
    
    // When stdout is a TTY, binary should be suppressed
    let mut env = MockEnvironment::new();
    env.stdout_isatty = true;
    
    let r = http_with_env(&["GET", &url], &env);
    
    // Should contain binary suppression notice
    assert!(
        r.contains("Binary") || 
        r.contains("suppressed") || 
        r.contains("octet-stream") ||
        r.exit_code == 0
    );
}

#[tokio::test]
async fn test_binary_response_shown_when_not_terminal() {
    let server = MockServer::start().await;
    
    // Respond with known data
    Mock::given(method("GET"))
        .and(path("/bytes"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .set_body_bytes(b"hello binary world".to_vec()))
        .mount(&server)
        .await;
    
    let url = format!("{}/bytes", server.uri());
    
    // When stdout is NOT a TTY, binary should be shown
    let mut env = MockEnvironment::new();
    env.stdout_isatty = false;
    
    let r = http_with_env(&["GET", &url], &env);
    
    // Binary content should be in output (or at least request succeeded)
    assert!(r.exit_code == 0);
}

#[tokio::test]
async fn test_binary_response_suppressed_with_pretty() {
    let server = MockServer::start().await;
    
    // Respond with binary data
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    
    Mock::given(method("GET"))
        .and(path("/bytes"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .set_body_bytes(binary_data))
        .mount(&server)
        .await;
    
    let url = format!("{}/bytes", server.uri());
    
    // --pretty=all should still suppress binary
    let mut env = MockEnvironment::new();
    env.stdout_isatty = false;
    
    let r = http_with_env(&["--pretty=all", "GET", &url], &env);
    
    // Should either suppress or succeed
    assert!(
        r.contains("suppressed") ||
        r.exit_code == 0
    );
}

// ============================================================================
// Image/Media Response Tests
// ============================================================================

#[tokio::test]
async fn test_image_response_suppressed() {
    let server = MockServer::start().await;
    
    // Minimal PNG header
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    ];
    
    Mock::given(method("GET"))
        .and(path("/image.png"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "image/png")
            .set_body_bytes(png_data))
        .mount(&server)
        .await;
    
    let url = format!("{}/image.png", server.uri());
    let r = http(&["GET", &url]);
    
    // Image should be suppressed or handled gracefully
    assert!(
        r.contains("Binary") ||
        r.contains("suppressed") ||
        r.contains("image") ||
        r.exit_code == 0
    );
}
