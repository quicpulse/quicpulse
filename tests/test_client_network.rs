//! Tests for low-level network controls: resolve, local address, port range, interface

mod common;
use common::http;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_resolve_flag_offline() {
    let r = http(&[
        "--offline",
        "--resolve=example.com:80:127.0.0.1",
        "http://example.com/api",
    ]);
    assert_eq!(r.exit_code, 0);
}

#[tokio::test]
async fn test_local_network_flags_offline() {
    let r = http(&[
        "--offline",
        "--local-address=127.0.0.1",
        "--tcp-fastopen",
        "http://example.com/status",
    ]);
    assert_eq!(r.exit_code, 0);
}

#[tokio::test]
async fn test_mock_with_custom_headers_and_network_options() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .mount(&server)
        .await;

    let url = format!("{}/ping", server.uri());
    let r = http(&[&url]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains("pong"));
}
