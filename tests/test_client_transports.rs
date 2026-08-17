//! Tests for client transports: HTTP versions, Unix sockets, and protocols

mod common;
use common::http;
use quicpulse::client::unix_socket::UnixSocketResponse;
use std::collections::HashMap;

#[test]
fn test_unix_socket_response_helpers() {
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/json; charset=utf-8".to_string(),
    );
    headers.insert("Server".to_string(), "Docker/24.0.5".to_string());

    let resp = UnixSocketResponse {
        status: 200,
        status_text: "OK".to_string(),
        headers,
        body: b"{\"version\": \"1.0\"}".to_vec(),
        http_version: "HTTP/1.1".to_string(),
    };

    assert_eq!(
        resp.header("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(resp.header("SERVER"), Some("Docker/24.0.5"));
    assert_eq!(resp.header("missing"), None);
    assert_eq!(resp.content_type(), Some("application/json; charset=utf-8"));
    assert_eq!(resp.text().unwrap(), "{\"version\": \"1.0\"}");
}

#[test]
fn test_http_version_flags_offline() {
    let r1 = http(&["--offline", "--http-version=1.1", "http://example.com/v1"]);
    assert_eq!(r1.exit_code, 0);

    let r2 = http(&["--offline", "--http-version=2", "https://example.com/v2"]);
    assert_eq!(r2.exit_code, 0);

    let r3 = http(&["--offline", "--http3", "https://example.com/v3"]);
    assert_eq!(r3.exit_code, 0);
}
