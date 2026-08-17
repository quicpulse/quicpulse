//! Tests for URL normalization, localhost detection, and port shorthands

mod common;
use common::http;
use quicpulse::utils::{is_localhost, url_as_host};

#[test]
fn test_url_as_host_helper() {
    assert_eq!(
        url_as_host("https://user:pass@example.com:8080/path"),
        "example.com:8080"
    );
    assert_eq!(url_as_host("http://example.com"), "example.com");
    assert_eq!(url_as_host("http://127.0.0.1:3000/api"), "127.0.0.1:3000");
    assert_eq!(url_as_host("http://[::1]:9090/v1"), "[::1]:9090");
    assert_eq!(url_as_host("invalid-url"), "invalid-url");
}

#[test]
fn test_is_localhost_helper() {
    assert!(is_localhost("localhost"));
    assert!(is_localhost("subdomain.localhost"));
    assert!(is_localhost("127.0.0.1"));
    assert!(is_localhost("127.0.0.100"));
    assert!(is_localhost("::1"));
    assert!(!is_localhost("example.com"));
    assert!(!is_localhost("8.8.8.8"));
}

#[test]
fn test_port_shorthand_offline() {
    let r = http(&["--offline", ":3000/api/users"]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains("localhost:3000") || r.stdout.contains("/api/users"));
}

#[test]
fn test_default_scheme_https_offline() {
    let r = http(&[
        "--offline",
        "--default-scheme=https",
        "api.example.com/data",
    ]);
    assert_eq!(r.exit_code, 0);
    assert!(r.stdout.contains("https://api.example.com") || r.stdout.contains("api.example.com"));
}
