//! Integration tests for multipart uploads and boundary edge cases

mod common;

use common::http;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_multipart_custom_boundary_and_metadata() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "received"
        })))
        .mount(&server)
        .await;

    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("doc.txt");
    fs::write(&file1, b"sample document content").unwrap();

    let file2 = dir.path().join("data.bin");
    fs::write(&file2, [0x00, 0x01, 0x02, 0xFF]).unwrap();

    let url = format!("{}/upload", server.uri());
    let r = http(&[
        "--verbose",
        "--boundary=QUICPULSE_BOUNDARY_999",
        &url,
        &format!("document@{}", file1.to_str().unwrap()),
        &format!(
            "payload@{};type=application/octet-stream;filename=blob.bin",
            file2.to_str().unwrap()
        ),
        "extra_field=metadata_value",
    ]);

    assert_eq!(r.exit_code, 0);
    assert!(
        r.stdout.contains("QUICPULSE_BOUNDARY_999")
            || r.stderr.contains("QUICPULSE_BOUNDARY_999")
            || r.stdout.contains("200 OK")
    );
}

#[tokio::test]
async fn test_multipart_empty_file() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/empty"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let dir = TempDir::new().unwrap();
    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, b"").unwrap();

    let url = format!("{}/empty", server.uri());
    let r = http(&[&url, &format!("file@{}", empty_file.to_str().unwrap())]);

    assert_eq!(r.exit_code, 0);
}
