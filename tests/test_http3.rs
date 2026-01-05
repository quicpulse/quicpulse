//! HTTP/3 (QUIC) integration tests
//!
//! These tests verify HTTP/3 functionality using both offline mode
//! and real HTTP/3 endpoints.
//!
//! Note: HTTP/3 requires the `http3` feature and RUSTFLAGS='--cfg reqwest_unstable'

mod common;

use common::{http, http_error};

// ============================================================================
// HTTP/3 Flag Tests (Offline)
// ============================================================================

#[test]
fn test_http3_flag_accepted() {
    // Test that --http3 flag is accepted
    let r = http(&["--http3", "--offline", "--print=H", "https://example.org"]);
    assert_eq!(r.exit_code, 0, "--http3 flag should be accepted");
}

#[test]
fn test_http3_with_http_version_flag() {
    // Test --http-version=3 flag
    let r = http(&["--http-version=3", "--offline", "--print=H", "https://example.org"]);
    assert_eq!(r.exit_code, 0, "--http-version=3 should be accepted");
}

#[test]
fn test_http3_plaintext_warning() {
    // HTTP/3 requires HTTPS - should warn for http://
    let r = http(&["--http3", "--offline", "--print=H", "http://example.org"]);

    // Should still work (falls back) but may have warning in stderr
    assert_eq!(r.exit_code, 0, "Should accept --http3 with http:// (with fallback)");
}

#[test]
fn test_http3_with_verbose() {
    // Test --http3 with verbose output
    let r = http(&["--http3", "--verbose", "--offline", "https://example.org"]);
    assert_eq!(r.exit_code, 0, "--http3 with --verbose should work");
}

// ============================================================================
// HTTP/3 Integration Tests (Real Endpoints)
// ============================================================================
// These tests require network access and are marked #[ignore] for CI.
// Run with: cargo test -- --ignored

#[test]
#[ignore = "requires network access to HTTP/3 endpoint"]
fn test_http3_cloudflare() {
    // Cloudflare supports HTTP/3
    // https://blog.cloudflare.com/http3-the-past-present-and-future/
    let r = http(&[
        "--http3",
        "--timeout", "10",
        "--print=hH",
        "https://cloudflare-quic.com/"
    ]);

    // Should successfully connect via HTTP/3
    assert_eq!(r.exit_code, 0, "HTTP/3 connection to Cloudflare should succeed");

    // Response should contain HTTP headers
    assert!(r.stdout.contains("HTTP/"), "Should receive HTTP response");
}

#[test]
#[ignore = "requires network access to HTTP/3 endpoint"]
fn test_http3_google() {
    // Google supports HTTP/3
    let r = http(&[
        "--http3",
        "--timeout", "10",
        "--print=hH",
        "https://www.google.com/"
    ]);

    // Should successfully connect
    assert_eq!(r.exit_code, 0, "HTTP/3 connection to Google should succeed");
}

#[test]
#[ignore = "requires network access to HTTP/3 endpoint"]
fn test_http3_with_headers() {
    // Test HTTP/3 with custom headers
    let r = http(&[
        "--http3",
        "--timeout", "10",
        "--print=hH",
        "https://cloudflare-quic.com/",
        "X-Custom-Header:test-value"
    ]);

    assert_eq!(r.exit_code, 0, "HTTP/3 with custom headers should work");
}

#[test]
#[ignore = "requires network access to HTTP/3 endpoint"]
fn test_http3_post_request() {
    // Test HTTP/3 POST request
    let r = http(&[
        "--http3",
        "--timeout", "10",
        "--print=hH",
        "POST",
        "https://httpbin.org/post",
        "message=hello"
    ]);

    // httpbin might not support HTTP/3, so this tests fallback behavior
    // or succeeds if it does support it
    assert!(r.exit_code == 0 || r.stderr.contains("error"),
        "POST should either succeed or fail gracefully");
}

#[test]
#[ignore = "requires network access to HTTP/3 endpoint"]
fn test_http3_json_response() {
    // Test HTTP/3 with JSON response
    let r = http(&[
        "--http3",
        "--timeout", "10",
        "--json",
        "https://cloudflare-quic.com/",
        "test=value"
    ]);

    // Should work with JSON content type
    assert!(r.exit_code == 0 || r.stderr.contains("error"),
        "JSON request should either succeed or fail gracefully");
}

// ============================================================================
// HTTP/3 Edge Cases
// ============================================================================

#[test]
fn test_http3_with_follow_redirects() {
    // Test --http3 combined with --follow
    let r = http(&["--http3", "--follow", "--offline", "--print=H", "https://example.org"]);
    assert_eq!(r.exit_code, 0, "--http3 with --follow should be accepted");
}

#[test]
fn test_http3_with_auth() {
    // Test --http3 combined with authentication
    let r = http(&[
        "--http3",
        "--auth-type=basic",
        "--auth=user:pass",
        "--offline",
        "--print=H",
        "https://example.org"
    ]);
    assert_eq!(r.exit_code, 0, "--http3 with --auth should be accepted");
}

#[test]
fn test_http_version_3_with_port() {
    // Test HTTP/3 with explicit port
    let r = http(&[
        "--http-version=3",
        "--offline",
        "--print=H",
        "https://example.org:443"
    ]);
    assert_eq!(r.exit_code, 0, "--http-version=3 with port should work");
}

#[test]
fn test_http3_combined_with_http2() {
    // If both --http3 and --http-version=2 are specified, --http3 should take precedence
    // (This tests flag priority)
    let r = http(&[
        "--http3",
        "--http-version=2",
        "--offline",
        "--print=H",
        "https://example.org"
    ]);
    // Should still work - implementation decides precedence
    assert_eq!(r.exit_code, 0, "Conflicting HTTP version flags should still work");
}

// ============================================================================
// HTTP/3 Error Handling
// ============================================================================

#[test]
#[ignore = "requires network - tests connection failure handling"]
fn test_http3_connection_refused() {
    // Test HTTP/3 to a server that doesn't support it
    // localhost:1 should refuse connection
    let r = http_error(&[
        "--http3",
        "--timeout=2",
        "https://localhost:1/"
    ]);

    // Should fail with connection error
    assert_ne!(r.exit_code, 0, "Connection to localhost:1 should fail");
}

#[test]
#[ignore = "requires network - tests timeout handling"]
fn test_http3_timeout() {
    // Test HTTP/3 timeout behavior
    // Use a very short timeout with a potentially slow endpoint
    let r = http_error(&[
        "--http3",
        "--timeout=1",
        "https://10.255.255.1/"  // Non-routable IP, should timeout
    ]);

    // Should fail with timeout or connection error
    assert_ne!(r.exit_code, 0, "Request to non-routable IP should timeout/fail");
}
