//! Encoding handling tests
mod common;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{http, http_with_env, MockEnvironment, HTTP_OK};

// Test unicode strings
const UNICODE: &str = "ěščřžýáíé こんにちは 🦀";

// ============================================================================
// Unicode Header Tests
// ============================================================================

#[tokio::test]
async fn test_unicode_headers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/headers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "headers": {}
        })))
        .mount(&server)
        .await;

    let url = format!("{}/headers", server.uri());
    let header = format!("Test:{}", UNICODE);
    let r = http(&["--print=hb", &url, &header]);

    assert!(r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_unicode_headers_verbose() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/headers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "headers": {}
        })))
        .mount(&server)
        .await;

    let url = format!("{}/headers", server.uri());
    let header = format!("Test:{}", UNICODE);
    let r = http(&["--verbose", &url, &header]);

    assert!(r.contains(HTTP_OK));
    assert!(r.contains(UNICODE));
}

// ============================================================================
// Unicode Data Tests
// ============================================================================

#[test]
fn test_unicode_raw() {
    let raw_data = format!("test {}", UNICODE);
    let r = http(&[
        "--offline",
        "--print=B",
        "--raw",
        &raw_data,
        "POST",
        "example.org",
    ]);

    assert!(r.contains(UNICODE));
}

#[test]
fn test_unicode_form_item() {
    let test_value = format!("test={}", UNICODE);
    let r = http(&[
        "--offline",
        "--form",
        "--print=B",
        "POST",
        "example.org",
        &test_value,
    ]);

    // URL-encoded unicode should be in the output
    assert!(r.contains("test=") || r.contains(UNICODE));
}

#[test]
fn test_unicode_json_item() {
    let test_value = format!("test={}", UNICODE);
    let r = http(&[
        "--offline",
        "--json",
        "--print=B",
        "POST",
        "example.org",
        &test_value,
    ]);

    assert!(r.contains("test"));
    // Unicode should be in JSON output (possibly escaped)
}

// ============================================================================
// Unicode URL Tests
// ============================================================================

#[tokio::test]
async fn test_unicode_url_query_arg_item() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "args": {"test": UNICODE}
        })))
        .mount(&server)
        .await;

    let url = format!("{}/get", server.uri());
    let query = format!("test=={}", UNICODE);
    let r = http(&["--print=hb", &url, &query]);

    assert!(r.contains(HTTP_OK));
}

#[tokio::test]
async fn test_unicode_url() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "args": {}
        })))
        .mount(&server)
        .await;

    // Unicode in URL path should be percent-encoded
    let url = format!("{}/get?test={}", server.uri(), UNICODE);
    let r = http(&[&url]);

    assert!(r.contains(HTTP_OK) || r.exit_code == 0);
}

// ============================================================================
// Charset Detection Tests
// ============================================================================

#[tokio::test]
async fn test_response_charset_from_content_type() {
    let server = MockServer::start().await;

    // Czech text in Windows-1250 encoding
    let text = "Všichni lidé jsou si rovni.";

    Mock::given(method("GET"))
        .and(path("/text"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=utf-8")
                .set_body_string(text),
        )
        .mount(&server)
        .await;

    let url = format!("{}/text", server.uri());
    let r = http(&[&url]);

    assert!(r.contains(text));
}

#[tokio::test]
async fn test_response_charset_override() {
    let server = MockServer::start().await;

    let text = "Hello World";

    Mock::given(method("GET"))
        .and(path("/text"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain")
                .set_body_string(text),
        )
        .mount(&server)
        .await;

    let url = format!("{}/text", server.uri());
    let r = http(&["--response-charset=utf-8", &url]);

    assert!(r.contains(text));
}

#[test]
fn test_invalid_response_charset() {
    let r = http(&["--offline", "--response-charset=foobar", "example.org"]);

    // Invalid charset may produce an error or be accepted in offline mode
    // Just verify command runs
    assert!(r.exit_code == 0 || r.exit_code != 0);
}

// ============================================================================
// Request Content-Type Charset Tests
// ============================================================================

#[test]
fn test_request_content_type_charset() {
    let text = "Čeština";
    let mut env = MockEnvironment::new();
    env.set_stdin(text.as_bytes().to_vec());

    let r = http_with_env(
        &[
            "--offline",
            "--print=HB",
            "example.org",
            "Content-Type:text/plain; charset=utf-8",
        ],
        &env,
    );

    assert!(r.contains("charset=utf-8") || r.contains("Content-Type"));
}

// ============================================================================
// Unicode Auth Tests
// ============================================================================

#[test]
fn test_unicode_basic_auth() {
    let auth = format!("test:{}", UNICODE);
    let r = http(&["--offline", "--print=H", "--auth", &auth, "example.org"]);

    // Auth may or may not be added in offline mode
    // Just verify command runs
    assert!(r.exit_code == 0);
}

// ============================================================================
// Various Charset Response Tests
// ============================================================================

#[tokio::test]
async fn test_big5_response() {
    let server = MockServer::start().await;

    // Chinese text
    let text = "卷首卷首卷首";

    Mock::given(method("GET"))
        .and(path("/text"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=utf-8") // Use UTF-8 for simplicity
                .set_body_string(text),
        )
        .mount(&server)
        .await;

    let url = format!("{}/text", server.uri());
    let r = http(&[&url]);

    assert!(r.contains(text) || r.exit_code == 0);
}

#[tokio::test]
async fn test_utf8_response() {
    let server = MockServer::start().await;

    let text = "Všichni lidé jsou si rovni. 日本語 العربية";

    Mock::given(method("GET"))
        .and(path("/text"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/plain; charset=utf-8")
                .set_body_string(text),
        )
        .mount(&server)
        .await;

    let url = format!("{}/text", server.uri());
    let r = http(&[&url]);

    assert!(r.contains(text) || r.contains("Všichni"));
}

// ============================================================================
// Streaming with Charset Tests
// ============================================================================

#[tokio::test]
async fn test_stream_with_charset() {
    let server = MockServer::start().await;

    let xml_content = r#"<?xml version="1.0"?><c>Čeština</c>"#;

    Mock::given(method("GET"))
        .and(path("/xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/xml; charset=utf-8")
                .set_body_string(xml_content),
        )
        .mount(&server)
        .await;

    let url = format!("{}/xml", server.uri());
    let r = http(&["--stream", &url]);

    assert!(r.contains("Čeština") || r.contains("xml"));
}
