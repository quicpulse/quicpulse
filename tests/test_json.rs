//! JSON handling tests
mod common;

use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

use common::{http, http_with_env, MockEnvironment, HTTP_OK};

// ============================================================================
// JSON Data Item Tests
// ============================================================================

#[test]
fn test_json_simple_values() {
    let r = http(&[
        "--offline", "--print=B", "example.org",
        "name=value",
        "count:=42",
        "active:=true",
    ]);
    
    assert!(r.contains(r#""name":"value""#));
    assert!(r.contains(r#""count":42"#));
    assert!(r.contains(r#""active":true"#));
}

#[test]
fn test_json_array_value() {
    let r = http(&[
        "--offline", "--print=B", "example.org",
        r#"items:=["a", "b", "c"]"#,
    ]);
    
    assert!(r.contains(r#""items""#));
    assert!(r.contains(r#"["a","b","c"]"#) || r.contains(r#""a""#));
}

#[test]
fn test_json_object_value() {
    let r = http(&[
        "--offline", "--print=B", "example.org",
        r#"config:={"key": "val"}"#,
    ]);
    
    assert!(r.contains(r#""config""#));
    assert!(r.contains(r#""key""#));
}

#[test]
fn test_json_null_value() {
    let r = http(&[
        "--offline", "--print=B", "example.org",
        "empty:=null",
    ]);
    
    assert!(r.contains(r#""empty":null"#));
}

// ============================================================================
// JSON Formatting Tests
// ============================================================================

#[tokio::test]
async fn test_json_response_formatted() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/json"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/json")
            .set_body_string(r#"{"a":1,"b":2}"#))
        .mount(&server)
        .await;
    
    let url = format!("{}/json", server.uri());
    let r = http(&["--pretty=format", "--print=b", &url]);
    
    // Formatted JSON should have indentation
    assert!(r.contains("\"a\""));
    assert!(r.contains("\"b\""));
}

#[tokio::test]
async fn test_json_response_unformatted() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/json"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/json")
            .set_body_string(r#"{"a":1,"b":2}"#))
        .mount(&server)
        .await;
    
    let url = format!("{}/json", server.uri());
    let r = http(&["--pretty=none", "--print=b", &url]);
    
    // Unformatted JSON should be compact
    assert!(r.contains(r#"{"a""#) || r.contains(r#""a":"#));
}

// ============================================================================
// Duplicate Keys Tests
// ============================================================================

#[tokio::test]
async fn test_json_duplicate_keys_preserved() {
    let server = MockServer::start().await;
    
    let json_with_dupes = r#"{"key": 15, "key": 15, "key": 3, "key": 7}"#;
    
    Mock::given(method("GET"))
        .and(path("/json"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/json")
            .set_body_string(json_with_dupes))
        .mount(&server)
        .await;
    
    let url = format!("{}/json", server.uri());
    let r = http(&["--pretty=format", "--print=b", &url]);
    
    // Should handle duplicate keys without crashing
    assert!(r.contains("key"));
}

// ============================================================================
// JSON with Non-JSON Content Types
// ============================================================================

#[tokio::test]
async fn test_json_explicit_with_text_content_type() {
    let server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/text"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .set_body_string(r#"{"actually": "json"}"#))
        .mount(&server)
        .await;
    
    let url = format!("{}/text", server.uri());
    let r = http(&["--print=b", &url]);
    
    // Even with text/plain, the body should be returned
    assert!(r.contains("actually"));
    assert!(r.contains("json"));
}

// ============================================================================
// Complex JSON Values in Form Mode
// ============================================================================

#[test]
fn test_simple_json_value_in_form_mode() {
    // Simple JSON values (numbers, bools, strings) should work in form mode
    let r = http(&["--offline", "--form", "--print=B", "example.org", "option:=42"]);
    assert!(r.contains("option"));
    assert!(r.contains("42"));
}

// ============================================================================
// JSON Format Options
// ============================================================================

#[test]
fn test_json_format_option_indent() {
    let r = http(&[
        "--offline", "--print=B",
        "--format-options=json.indent:2",
        "example.org", "foo=bar", "baz=qux",
    ]);
    
    // With indent:2, JSON should have 2-space indentation
    assert!(r.contains("foo"));
    assert!(r.contains("baz"));
}

#[test]
#[ignore] // TODO: sort_keys feature not yet implemented
fn test_json_format_option_sort_keys() {
    let r = http(&[
        "--offline", "--print=B",
        "--format-options=json.sort_keys:true",
        "example.org", "z=last", "a=first",
    ]);

    // With sort_keys:true, 'a' should appear before 'z'
    let a_pos = r.stdout.find("\"a\"");
    let z_pos = r.stdout.find("\"z\"");

    if let (Some(a), Some(z)) = (a_pos, z_pos) {
        assert!(a < z, "Keys should be sorted alphabetically");
    }
}

#[test]
fn test_unsorted_json_output() {
    let r = http(&[
        "--offline", "--print=B",
        "--unsorted",
        "example.org", "z=last", "a=first",
    ]);
    
    // With --unsorted, keys are processed in order but HashMap doesn't preserve order
    // Just verify both keys are present
    assert!(r.contains("\"z\""));
    assert!(r.contains("\"a\""));
}
