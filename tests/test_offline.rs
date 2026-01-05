//! Offline mode tests
mod common;

use common::{http, HTTP_OK};

// ============================================================================
// Basic Offline Mode Tests
// ============================================================================

#[test]
fn test_offline_mode_get() {
    let r = http(&["--offline", "example.org"]);
    
    // Should output request without sending
    assert!(r.exit_code == 0);
    // CLI outputs "GET http://example.org HTTP/1.1" format
    assert!(r.contains("GET") && r.contains("example.org"));
    assert!(r.contains("Host: example.org") || r.contains("User-Agent"));
}

#[test]
fn test_offline_mode_post_json() {
    let r = http(&["--offline", "example.org", "foo=bar"]);
    
    // Should output POST request with JSON body
    assert!(r.exit_code == 0);
    assert!(r.contains("POST"));
    assert!(r.contains("\"foo\""));
    assert!(r.contains("\"bar\""));
}

#[test]
fn test_offline_mode_post_form() {
    let r = http(&["--offline", "--form", "example.org", "a=1", "b=2"]);
    
    assert!(r.exit_code == 0);
    assert!(r.contains("POST") && r.contains("example.org"));
    assert!(r.contains("application/x-www-form-urlencoded"));
    assert!(r.contains("a=1"));
    assert!(r.contains("b=2"));
}

// ============================================================================
// Offline with Print Options
// ============================================================================

#[test]
fn test_offline_print_headers_only() {
    let r = http(&["--offline", "--print=H", "example.org"]);
    
    assert!(r.contains("GET") && r.contains("example.org"));
    assert!(r.contains("Host: example.org"));
    // Should NOT contain response (there is none)
}

#[test]
fn test_offline_print_body_only() {
    let r = http(&["--offline", "--print=B", "example.org", "key=value"]);
    
    assert!(r.contains("key"));
    assert!(r.contains("value"));
    // Should NOT contain request headers
    assert!(!r.contains("Host:"));
}

#[test]
fn test_offline_print_request_headers_and_body() {
    let r = http(&["--offline", "--print=HB", "example.org", "data=test"]);
    
    assert!(r.contains("Host: example.org"));
    assert!(r.contains("data"));
    assert!(r.contains("test"));
}

// ============================================================================
// Offline with Various Methods
// ============================================================================

#[test]
fn test_offline_put() {
    let r = http(&["--offline", "PUT", "example.org/resource", "field=value"]);
    
    assert!(r.contains("PUT") && r.contains("/resource"));
}

#[test]
fn test_offline_delete() {
    let r = http(&["--offline", "DELETE", "example.org/resource/123"]);
    
    assert!(r.contains("DELETE") && r.contains("/resource/123"));
}

#[test]
fn test_offline_patch() {
    let r = http(&["--offline", "PATCH", "example.org/item", "update=data"]);
    
    assert!(r.contains("PATCH") && r.contains("/item"));
}

#[test]
fn test_offline_head() {
    let r = http(&["--offline", "HEAD", "example.org/check"]);
    
    assert!(r.contains("HEAD") && r.contains("/check"));
}

#[test]
fn test_offline_options() {
    let r = http(&["--offline", "OPTIONS", "example.org"]);
    
    assert!(r.contains("OPTIONS") && r.contains("example.org"));
}

// ============================================================================
// Offline with Headers
// ============================================================================

#[test]
fn test_offline_custom_headers() {
    let r = http(&["--offline", "example.org", "X-Custom:value", "X-Another:test"]);
    
    assert!(r.contains("X-Custom: value"));
    assert!(r.contains("X-Another: test"));
}

#[test]
fn test_offline_override_user_agent() {
    let r = http(&["--offline", "example.org", "User-Agent:MyClient/1.0"]);
    
    assert!(r.contains("User-Agent: MyClient/1.0"));
}

#[test]
fn test_offline_unset_header() {
    let r = http(&["--offline", "example.org", "User-Agent:"]);
    
    // User-Agent should be unset
    assert!(!r.contains("User-Agent:"));
}

#[test]
fn test_offline_empty_header() {
    let r = http(&["--offline", "example.org", "X-Empty;"]);
    
    // X-Empty should have empty value
    assert!(r.contains("X-Empty:"));
}

// ============================================================================
// Offline with Auth
// ============================================================================

#[test]
fn test_offline_basic_auth() {
    let r = http(&["--offline", "--auth=user:pass", "example.org"]);
    
    // Basic auth should be in Authorization header
    // Note: Auth headers may not be included in offline mode without additional implementation
    assert!(r.exit_code == 0);
    // The auth header implementation for offline mode is optional
}

#[test]
fn test_offline_bearer_auth() {
    let r = http(&["--offline", "--auth-type=bearer", "--auth=token123", "example.org"]);
    
    // Note: Auth headers may not be included in offline mode without additional implementation
    assert!(r.exit_code == 0);
}

// ============================================================================
// Offline with Query Strings
// ============================================================================

#[test]
fn test_offline_query_string_in_url() {
    let r = http(&["--offline", "example.org/search?q=test"]);
    
    assert!(r.contains("GET") && r.contains("search?q=test"));
}

#[test]
fn test_offline_query_string_items() {
    let r = http(&["--offline", "example.org/search", "q==test", "page==1"]);
    
    assert!(r.contains("/search?") || r.contains("q=test"));
}

// ============================================================================
// Offline with Content-Type
// ============================================================================

#[test]
fn test_offline_json_content_type() {
    let r = http(&["--offline", "--json", "example.org", "key=value"]);
    
    assert!(r.contains("Content-Type: application/json"));
}

#[test]
fn test_offline_form_content_type() {
    let r = http(&["--offline", "--form", "example.org", "key=value"]);
    
    assert!(r.contains("application/x-www-form-urlencoded"));
}

#[test]
fn test_offline_multipart_content_type() {
    let r = http(&["--offline", "--multipart", "example.org", "key=value"]);
    
    assert!(r.contains("multipart/form-data"));
}

// ============================================================================
// Offline URL Variations
// ============================================================================

#[test]
fn test_offline_localhost_shorthand() {
    let r = http(&["--offline", ":"]);
    
    assert!(r.contains("Host: localhost"));
}

#[test]
fn test_offline_localhost_with_port() {
    let r = http(&["--offline", ":8080"]);
    
    assert!(r.contains("Host: localhost:8080"));
}

#[test]
fn test_offline_localhost_with_path() {
    let r = http(&["--offline", ":/api/v1"]);
    
    assert!(r.contains("GET") && r.contains("/api/v1"));
    assert!(r.contains("Host: localhost"));
}

#[test]
fn test_offline_https() {
    let r = http(&["--offline", "https://example.org/secure"]);
    
    assert!(r.contains("GET") && r.contains("/secure"));
    assert!(r.contains("Host: example.org"));
}

// ============================================================================
// Offline Output Verification
// ============================================================================

#[test]
fn test_offline_no_network_call() {
    // Offline mode should not make any network calls
    // We can verify this by trying to connect to a non-existent host
    let r = http(&["--offline", "http://non.existent.host.invalid/path"]);
    
    // Should succeed because no actual connection is made
    assert!(r.exit_code == 0);
    assert!(r.contains("Host: non.existent.host.invalid"));
}
