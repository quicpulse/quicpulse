//! Integration tests for client resilience: timeouts, redirects, check-status

mod common;

use common::http_error;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_check_status_404_failure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/not-found"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let url = format!("{}/not-found", server.uri());
    let r = http_error(&["--check-status", &url]);
    assert_ne!(
        r.exit_code, 0,
        "Expected non-zero exit code with --check-status on 404"
    );
}

#[tokio::test]
async fn test_check_status_500_failure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/server-error"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let url = format!("{}/server-error", server.uri());
    let r = http_error(&["--check-status", &url]);
    assert_ne!(
        r.exit_code, 0,
        "Expected non-zero exit code with --check-status on 500"
    );
}

#[tokio::test]
async fn test_client_timeout_triggers_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(800))
                .set_body_string("slow response"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/slow", server.uri());
    let r = http_error(&["--timeout=0.1", &url]);
    assert_ne!(
        r.exit_code, 0,
        "Expected timeout error with 100ms timeout on 800ms delay"
    );
}
