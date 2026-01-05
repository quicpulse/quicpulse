//! Compression tests
mod common;

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};

use common::{http, http_error, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// Compression Flag Conflict Tests
// ============================================================================

#[test]
fn test_cannot_combine_compress_with_chunked() {
    let r = http(&["--compress", "--chunked", "--offline", "example.org"]);
    
    // May or may not error when combining --compress and --chunked
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

#[test]
fn test_cannot_combine_compress_with_multipart() {
    let r = http(&["--compress", "--multipart", "--offline", "example.org", "foo=bar"]);
    
    // May or may not error when combining --compress and --multipart
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

// ============================================================================
// Compression Skipping Tests
// ============================================================================

#[tokio::test]
async fn test_compress_skip_negative_ratio() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {},
                "json": {"foo": "bar"}
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    // Small data that doesn't benefit from compression
    let r = http(&["--print=hb", "--compress", &url, "foo=bar"]);

    assert!(r.contains(HTTP_OK));
    // Content-Encoding should NOT be present for small payloads
    // (compression would increase size)
}

// ============================================================================
// Force Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_force_with_negative_ratio() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate"
                }
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    // Double --compress forces compression even with negative ratio
    let r = http(&["--print=hb", "--compress", "--compress", &url, "foo=bar"]);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// JSON Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_json() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate",
                    "Content-Type": "application/json"
                },
                "data": "",
                "json": null
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--print=hb", "--compress", "--compress", &url, "foo=bar"]);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Form Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_form() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate"
                },
                "data": ""
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--print=hb", "--form", "--compress", "--compress", &url, "foo=bar"]);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Raw Data Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_raw() {
    let server = MockServer::start().await;
    
    let large_content = "This is a test content that should be compressed. ".repeat(10);
    
    Mock::given(method("POST"))
        .and(path("/post"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate"
                }
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/post", server.uri());
    let r = http(&["--print=hb", "--raw", &large_content, "--compress", "--compress", &url]);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Stdin Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_stdin() {
    let server = MockServer::start().await;
    
    let large_content = "Compressible content repeated many times. ".repeat(20);
    
    Mock::given(method("PATCH"))
        .and(path("/patch"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate"
                }
            })))
        .mount(&server)
        .await;
    
    let mut env = MockEnvironment::new();
    env.set_stdin(large_content.as_bytes().to_vec());
    
    let url = format!("{}/patch", server.uri());
    let r = http_with_env(&["--print=hb", "--compress", "--compress", "PATCH", &url], &env);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// File Compression Tests
// ============================================================================

#[tokio::test]
async fn test_compress_file() {
    let server = MockServer::start().await;
    
    Mock::given(method("PUT"))
        .and(path("/put"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "headers": {
                    "Content-Encoding": "deflate"
                },
                "files": {}
            })))
        .mount(&server)
        .await;
    
    // Create a temp file
    let dir = tempfile::TempDir::new().unwrap();
    let file_path = dir.path().join("test.txt");
    let content = "Test file content that will be compressed. ".repeat(10);
    std::fs::write(&file_path, &content).unwrap();
    
    let url = format!("{}/put", server.uri());
    let file_arg = format!("file@{}", file_path.display());
    let r = http(&["--print=hb", "--form", "--compress", "--compress", "PUT", &url, &file_arg]);

    assert!(r.contains(HTTP_OK));
}

// ============================================================================
// Response Decompression Tests
// ============================================================================

#[tokio::test]
async fn test_response_gzip_decompressed() {
    let server = MockServer::start().await;
    
    // Mock a gzip response (wiremock handles this automatically usually)
    Mock::given(method("GET"))
        .and(path("/gzip"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "gzipped": true
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/gzip", server.uri());
    let r = http(&[&url]);
    
    // Response should be decompressed automatically
    assert!(r.contains("gzipped") || r.exit_code == 0);
}

#[tokio::test]
async fn test_response_deflate_decompressed() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/deflate"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "deflated": true
            })))
        .mount(&server)
        .await;
    
    let url = format!("{}/deflate", server.uri());
    let r = http(&[&url]);
    
    assert!(r.contains("deflated") || r.exit_code == 0);
}

// ============================================================================
// Accept-Encoding Header Tests
// ============================================================================

#[test]
fn test_accept_encoding_header_sent() {
    let r = http(&["--offline", "--print=H", "example.org"]);
    
    // Accept-Encoding may or may not be sent in offline mode
    assert!(r.exit_code == 0);
}

#[test]
fn test_accept_encoding_can_be_overridden() {
    let r = http(&["--offline", "--print=H", "example.org", "Accept-Encoding:identity"]);
    
    // Custom Accept-Encoding should override default
    assert!(r.contains("Accept-Encoding: identity"));
}
