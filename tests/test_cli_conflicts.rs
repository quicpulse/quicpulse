//! Integration and unit tests for CLI conflicts and invalid argument combinations

use clap::Parser;
use quicpulse::cli::Args;

macro_rules! parse_args {
    ($($arg:expr),*) => {{
        Args::try_parse_from(["quicpulse", $($arg),*])
    }};
}

#[test]
fn test_headers_and_body_conflict() {
    let result = parse_args!("-h", "-b", "http://example.com");
    assert!(result.is_err(), "Expected -h and -b to conflict");
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("cannot be used with") || err_str.contains("conflict"));
}

#[test]
fn test_invalid_auth_type_enum() {
    let result = parse_args!("-A", "invalid_auth_type", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_invalid_pretty_option_enum() {
    let result = parse_args!("--pretty=unknown_style", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_invalid_log_format_enum() {
    let result = parse_args!("--log-format=yaml", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_invalid_timeout_number() {
    let result = parse_args!("--timeout=not_a_number", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_invalid_redirect_number() {
    let result = parse_args!("--max-redirects=not_a_number", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_invalid_oauth_port() {
    let result = parse_args!("--oauth-redirect-port=99999999", "http://example.com");
    assert!(result.is_err());
}

#[test]
fn test_valid_all_auth_types() {
    let types = [
        "basic",
        "digest",
        "bearer",
        "aws-sigv4",
        "aws",
        "gcp",
        "google",
        "azure",
        "az",
        "oauth2",
        "oauth",
        "oauth2-auth-code",
        "oauth-code",
        "oauth2-device",
        "oauth-device",
        "ntlm",
        "negotiate",
        "kerberos",
    ];
    for t in types {
        let result = parse_args!("-A", t, "http://example.com");
        assert!(
            result.is_ok(),
            "Auth type '{}' should parse successfully",
            t
        );
    }
}
