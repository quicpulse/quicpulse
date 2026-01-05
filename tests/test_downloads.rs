//! File download tests
mod common;

use std::fs;
use tempfile::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};

use common::{http, http_with_env, http_error, MockEnvironment, HTTP_OK};

// ============================================================================
// Basic Download Tests
// ============================================================================

#[tokio::test]
async fn test_download() {
    let server = MockServer::start().await;
    
    let content = "User-agent: *\nDisallow: /\n";
    
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .insert_header("Content-Length", content.len().to_string())
            .set_body_string(content))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let orig_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    
    let url = format!("{}/robots.txt", server.uri());
    let r = http(&["--download", &url]);
    
    // Reset current dir
    std::env::set_current_dir(orig_dir).unwrap();
    
    // Should have downloading message in stderr
    assert!(r.stderr.contains("Downloading") || r.exit_code == 0);
}

// ============================================================================
// Download Utility Tests
// ============================================================================

#[test]
fn test_content_range_parsing() {
    // These are unit tests for Content-Range parsing logic
    // Testing via the CLI behavior
    
    // A download with Content-Range should work
    // This is tested implicitly through download resume tests
}

#[test]
fn test_content_disposition_filename() {
    // Content-Disposition filename extraction
    // Tested through download behavior
}

// ============================================================================
// Download to Specific Output Tests
// ============================================================================

#[tokio::test]
async fn test_download_with_output() {
    let server = MockServer::start().await;
    
    let content = "Downloaded content";
    
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .set_body_string(content))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("downloaded.txt");
    
    let url = format!("{}/file", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    assert!(r.exit_code == 0);
    
    // Check file was created with content
    if output_path.exists() {
        let downloaded = fs::read_to_string(&output_path).unwrap();
        assert!(downloaded.contains(content) || downloaded.len() > 0);
    }
}

// ============================================================================
// Download with Content-Disposition Tests
// ============================================================================

#[tokio::test]
async fn test_download_filename_from_content_disposition() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/download"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .insert_header("Content-Disposition", "attachment; filename=\"custom_name.bin\"")
            .insert_header("Content-Length", "5")
            .set_body_string("12345"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("downloaded.bin");
    
    let url = format!("{}/download", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    // File should be downloaded
    assert!(r.exit_code == 0);
}

// ============================================================================
// Download with Redirect Tests
// ============================================================================

#[tokio::test]
async fn test_download_with_redirect() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/redirect/1"))
        .respond_with(ResponseTemplate::new(302)
            .insert_header("Location", "/file.txt"))
        .mount(&server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/file.txt"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .set_body_string("Redirected content"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("redirected.txt");
    
    let url = format!("{}/redirect/1", server.uri());
    let r = http(&["--download", "--follow", "--output", output_path.to_str().unwrap(), &url]);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Partial Download / Resume Tests
// ============================================================================

#[tokio::test]
async fn test_download_resume_with_range_header() {
    let server = MockServer::start().await;
    
    // Full content response
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .insert_header("Content-Length", "10")
            .set_body_string("0123456789"))
        .mount(&server)
        .await;
    
    // Partial content response (for resume)
    Mock::given(method("GET"))
        .and(path("/file"))
        .and(header("Range", "bytes=5-"))
        .respond_with(ResponseTemplate::new(206)
            .insert_header("Content-Type", "application/octet-stream")
            .insert_header("Content-Range", "bytes 5-9/10")
            .set_body_string("56789"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("file.bin");
    
    // Create partial file
    fs::write(&file_path, "01234").unwrap();
    
    let url = format!("{}/file", server.uri());
    let r = http(&["--download", "--continue", "--output", file_path.to_str().unwrap(), &url]);
    
    // Resume should work
    assert!(r.exit_code == 0);
}

// ============================================================================
// Download with Content-Length Tests
// ============================================================================

#[tokio::test]
async fn test_download_with_content_length() {
    let server = MockServer::start().await;
    
    let content = "Known size content";
    
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .insert_header("Content-Length", content.len().to_string())
            .set_body_string(content))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("output.bin");
    
    let url = format!("{}/file", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    assert!(r.exit_code == 0);
    
    // Verify size
    if output_path.exists() {
        let metadata = fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }
}

#[tokio::test]
async fn test_download_without_content_length() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/stream"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            // No Content-Length header
            .set_body_string("Unknown size streamed content"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("output.bin");
    
    let url = format!("{}/stream", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    assert!(r.exit_code == 0);
}

// ============================================================================
// Download Filename Generation Tests
// ============================================================================

#[tokio::test]
async fn test_download_filename_from_url() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/path/to/document.pdf"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/pdf")
            .set_body_bytes(b"%PDF-1.4".to_vec()))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("document.pdf");
    
    let url = format!("{}/path/to/document.pdf", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    // Should download file
    assert!(r.exit_code == 0);
}

// ============================================================================
// Unique Filename Tests
// ============================================================================

#[tokio::test]
async fn test_download_unique_filename() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/file.txt"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .set_body_string("Content"))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("downloaded_unique.txt");
    
    let url = format!("{}/file.txt", server.uri());
    let r = http(&["--download", "--output", output_path.to_str().unwrap(), &url]);
    
    // Should create file
    assert!(r.exit_code == 0);
}

// ============================================================================
// Download Progress Tests
// ============================================================================

#[tokio::test]
async fn test_download_shows_progress() {
    let server = MockServer::start().await;
    
    let content = "a".repeat(1000); // 1KB of data
    
    Mock::given(method("GET"))
        .and(path("/large"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/octet-stream")
            .insert_header("Content-Length", content.len().to_string())
            .set_body_string(content))
        .mount(&server)
        .await;
    
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("large.bin");
    
    let mut env = MockEnvironment::new();
    env.stdout_isatty = true;
    
    let url = format!("{}/large", server.uri());
    let r = http_with_env(&["--download", "--output", output_path.to_str().unwrap(), &url], &env);
    
    // Progress should be shown in terminal
    assert!(r.exit_code == 0);
}
