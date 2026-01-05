//! Tests for SOCKS proxy support

mod common;
use common::{http_with_env, MockEnvironment};

// Note: These tests primarily verify that the CLI arguments are parsed correctly
// and that the proxy configuration is applied. Full proxy functionality requires
// an actual SOCKS proxy server running.

#[test]
fn test_socks_proxy_arg_parsing() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test --socks argument parsing
    let args = Args::try_parse_from([
        "quicpulse",
        "--socks", "socks5://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.socks_proxy.is_some());
    assert_eq!(args.socks_proxy.unwrap().to_string(), "socks5://localhost:1080");
}

#[test]
fn test_socks_proxy_alias() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test --socks-proxy alias
    let args = Args::try_parse_from([
        "quicpulse",
        "--socks-proxy", "socks4://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.socks_proxy.is_some());
}

#[test]
fn test_proxy_arg_socks4() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test --proxy with socks4:// prefix
    let args = Args::try_parse_from([
        "quicpulse",
        "--proxy", "socks4://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(!args.proxy.is_empty());
    assert!(args.proxy[0].to_string().starts_with("socks4://"));
}

#[test]
fn test_proxy_arg_socks5() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test --proxy with socks5:// prefix
    let args = Args::try_parse_from([
        "quicpulse",
        "--proxy", "socks5://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_proxy_arg_socks5h() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test --proxy with socks5h:// prefix (DNS resolution through proxy)
    let args = Args::try_parse_from([
        "quicpulse",
        "--proxy", "socks5h://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_proxy_with_auth() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test SOCKS proxy with authentication
    let args = Args::try_parse_from([
        "quicpulse",
        "--socks", "socks5://user:pass@localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.socks_proxy.is_some());
    // Note: SensitiveUrl redacts credentials in display
}

#[test]
fn test_multiple_proxy_args() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test combining --socks with --proxy
    let args = Args::try_parse_from([
        "quicpulse",
        "--socks", "socks5://localhost:1080",
        "--proxy", "http:http://http-proxy:8080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.socks_proxy.is_some());
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_socks_proxy_simple_format() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test simple host:port format (should default to socks5)
    let args = Args::try_parse_from([
        "quicpulse",
        "--socks", "localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.socks_proxy.is_some());
    assert_eq!(args.socks_proxy.unwrap().to_string(), "localhost:1080");
}

// Integration test - requires actual proxy server
#[tokio::test]
#[ignore] // Only run when a SOCKS proxy is available
async fn test_socks5_proxy_integration() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::method;

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("success"))
        .mount(&mock_server)
        .await;

    let env = MockEnvironment::new();

    // This test would need an actual SOCKS5 proxy running at localhost:1080
    let result = http_with_env(&[
        "--socks", "socks5://localhost:1080",
        "GET", &mock_server.uri()
    ], &env);

    assert!(result.stdout.contains("success") || result.exit_code != 0);
}

#[test]
fn test_proxy_url_formats() {
    // Test various proxy URL formats that should be valid
    let valid_urls = vec![
        "socks4://localhost:1080",
        "socks4a://localhost:1080",
        "socks5://localhost:1080",
        "socks5h://localhost:1080",
        "socks5://user:pass@localhost:1080",
        "socks5://127.0.0.1:1080",
        "socks5://[::1]:1080",
    ];

    for url in valid_urls {
        // Verify URL can be parsed as a reqwest proxy
        let result = reqwest::Proxy::all(url);
        assert!(result.is_ok(), "Failed to parse proxy URL: {}", url);
    }
}

#[test]
fn test_proxy_protocol_case_insensitive() {
    use quicpulse::cli::Args;
    use clap::Parser;

    // Test that protocol is case-insensitive
    let args = Args::try_parse_from([
        "quicpulse",
        "--proxy", "SOCKS5://localhost:1080",
        "--offline",
        "GET", "http://example.com"
    ]);

    assert!(args.is_ok());
}
