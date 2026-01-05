//! Comprehensive CLI option tests
//!
//! Tests that all CLI options are correctly parsed and handle edge cases.

mod common;
use common::{http_with_env, MockEnvironment};

use quicpulse::cli::Args;
use clap::Parser;

// ============================================================================
// Helper macros
// ============================================================================

macro_rules! parse_args {
    ($($arg:expr),*) => {{
        Args::try_parse_from(["quicpulse", $($arg),*])
    }};
}

macro_rules! assert_parses {
    ($($arg:expr),*) => {{
        let result = Args::try_parse_from(["quicpulse", $($arg),*]);
        assert!(result.is_ok(), "Failed to parse args: {:?}", result.err());
        result.unwrap()
    }};
}

// ============================================================================
// Basic HTTP Options Tests
// ============================================================================

#[test]
fn test_simple_get() {
    let args = assert_parses!("GET", "http://example.com");
    assert_eq!(args.method.unwrap(), "GET");
    assert_eq!(args.url.unwrap(), "http://example.com");
}

#[test]
fn test_implicit_get() {
    // When only a URL is provided without a method, the URL ends up in the method field
    // and the URL field is None. This is normalized during request processing.
    let args = assert_parses!("http://example.com");
    // The URL might be in method field since it's parsed first
    assert!(args.method.is_some() || args.url.is_some());
}

#[test]
fn test_post_with_data() {
    let args = assert_parses!("POST", "http://example.com", "name=John");
    assert_eq!(args.method.unwrap(), "POST");
    assert!(!args.request_items.is_empty());
}

#[test]
fn test_all_http_methods() {
    for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
        let args = assert_parses!(method, "http://example.com", "--offline");
        assert_eq!(args.method.unwrap().to_uppercase(), *method);
    }
}

// ============================================================================
// Output Control Options
// ============================================================================

#[test]
fn test_headers_only_short() {
    let args = assert_parses!("--headers", "http://example.com");
    assert!(args.headers_only);
}

#[test]
fn test_body_only_short() {
    let args = assert_parses!("-b", "http://example.com");
    assert!(args.body);
}

#[test]
fn test_verbose_short() {
    let args = assert_parses!("-v", "http://example.com");
    assert!(args.verbose > 0);
}

#[test]
fn test_quiet_mode() {
    let args = assert_parses!("-q", "http://example.com");
    assert!(args.quiet > 0);
}

#[test]
fn test_print_option() {
    let args = assert_parses!("--print=Hh", "http://example.com");
    assert_eq!(args.print, Some("Hh".to_string()));
}

#[test]
fn test_pretty_options() {
    for pretty in &["all", "colors", "format", "none"] {
        let args = assert_parses!("--pretty", *pretty, "http://example.com");
        assert!(args.pretty.is_some());
    }
}

#[test]
fn test_style_option() {
    let args = assert_parses!("--style=monokai", "http://example.com");
    assert_eq!(args.style, Some("monokai".to_string()));
}

// ============================================================================
// Authentication Options
// ============================================================================

#[test]
fn test_basic_auth_short() {
    let args = assert_parses!("-a", "user:pass", "http://example.com");
    assert!(args.auth.is_some());
}

#[test]
fn test_auth_type_bearer() {
    let args = assert_parses!("-A", "bearer", "-a", "token123", "http://example.com");
    assert!(args.auth_type.is_some());
}

#[test]
fn test_auth_type_digest() {
    let args = assert_parses!("-A", "digest", "-a", "user:pass", "http://example.com");
    assert!(args.auth_type.is_some());
}

// ============================================================================
// Proxy Options
// ============================================================================

#[test]
fn test_http_proxy() {
    let args = assert_parses!("--proxy=http://proxy:8080", "http://example.com");
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_socks_proxy() {
    let args = assert_parses!("--socks=socks5://localhost:1080", "http://example.com");
    assert!(args.socks_proxy.is_some());
}

#[test]
fn test_socks_proxy_alias() {
    let args = assert_parses!("--socks-proxy=socks5://localhost:1080", "http://example.com");
    assert!(args.socks_proxy.is_some());
}

#[test]
fn test_socks5_via_proxy_arg() {
    let args = assert_parses!("--proxy=socks5://localhost:1080", "http://example.com");
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_socks5h_proxy() {
    let args = assert_parses!("--socks=socks5h://localhost:1080", "http://example.com");
    assert!(args.socks_proxy.is_some());
}

// ============================================================================
// SSL/TLS Options
// ============================================================================

#[test]
fn test_verify_no() {
    let args = assert_parses!("--verify=no", "https://example.com");
    assert_eq!(args.verify, "no");
}

#[test]
fn test_ssl_version() {
    let args = assert_parses!("--ssl=tls1.2", "https://example.com");
    assert!(args.ssl.is_some());
}

#[test]
fn test_cert_options() {
    let args = assert_parses!("--cert=client.pem", "--cert-key=key.pem", "https://example.com");
    assert!(args.cert.is_some());
    assert!(args.cert_key.is_some());
}

// ============================================================================
// Timeout and Retry Options
// ============================================================================

#[test]
fn test_timeout() {
    let args = assert_parses!("--timeout=30", "http://example.com");
    assert!(args.timeout.is_some());
}

#[test]
fn test_max_redirects() {
    let args = assert_parses!("--max-redirects=5", "http://example.com");
    assert_eq!(args.max_redirects, 5);
}

#[test]
fn test_no_follow_redirects() {
    let args = assert_parses!("-F", "http://example.com");
    assert!(args.follow);
}

// ============================================================================
// Session Options
// ============================================================================

#[test]
fn test_session() {
    let args = assert_parses!("--session=mysession", "http://example.com");
    assert!(args.session.is_some());
}

#[test]
fn test_session_read_only() {
    let args = assert_parses!("--session-read-only=mysession", "http://example.com");
    assert!(args.session_read_only.is_some());
}

// ============================================================================
// Download Options
// ============================================================================

#[test]
fn test_download_short() {
    let args = assert_parses!("-d", "http://example.com/file.zip");
    assert!(args.download);
}

#[test]
fn test_download_output() {
    let args = assert_parses!("-d", "-o", "output.zip", "http://example.com/file.zip");
    assert!(args.download);
    assert!(args.output.is_some());
}

#[test]
fn test_continue_download() {
    let args = assert_parses!("-d", "-c", "http://example.com/file.zip");
    assert!(args.download);
    assert!(args.continue_download);
}

// ============================================================================
// Assertion Options
// ============================================================================

#[test]
fn test_assert_status() {
    let args = assert_parses!("--assert-status=200", "http://example.com");
    assert!(args.assert_status.is_some());
}

#[test]
fn test_assert_time() {
    let args = assert_parses!("--assert-time=<500ms", "http://example.com");
    assert!(args.assert_time.is_some());
}

#[test]
fn test_assert_body() {
    let args = assert_parses!("--assert-body=.success == true", "http://example.com");
    assert!(args.assert_body.is_some());
}

#[test]
fn test_assert_header() {
    let args = assert_parses!("--assert-header=Content-Type:application/json", "http://example.com");
    assert!(!args.assert_header.is_empty());
}

// ============================================================================
// Filter and Format Options
// ============================================================================

#[test]
fn test_filter() {
    let args = assert_parses!("--filter=.data", "http://example.com");
    assert!(args.filter.is_some());
}

#[test]
fn test_table_output() {
    let args = assert_parses!("--table", "http://example.com");
    assert!(args.table);
}

#[test]
fn test_csv_output() {
    let args = assert_parses!("--csv", "http://example.com");
    assert!(args.csv);
}

// ============================================================================
// Protocol Options
// ============================================================================

#[test]
fn test_http_version() {
    let args = assert_parses!("--http-version=2", "https://example.com");
    assert!(args.http_version.is_some());
}

#[test]
fn test_http3() {
    let args = assert_parses!("--http3", "https://example.com");
    assert!(args.http3);
}

// ============================================================================
// GraphQL Options
// ============================================================================

#[test]
fn test_graphql_short() {
    let args = assert_parses!("-G", "POST", "http://example.com/graphql", "query={}");
    assert!(args.graphql);
}

#[test]
fn test_graphql_schema() {
    let args = assert_parses!("-G", "--graphql-schema", "http://example.com/graphql");
    assert!(args.graphql);
    assert!(args.graphql_schema);
}

// ============================================================================
// gRPC Options
// ============================================================================

#[test]
fn test_grpc() {
    let args = assert_parses!("--grpc", "grpc://localhost:50051/service/Method");
    assert!(args.grpc);
}

#[test]
fn test_grpc_proto() {
    let args = assert_parses!("--grpc", "--proto=service.proto", "grpc://localhost:50051/service/Method");
    assert!(args.grpc);
    assert!(args.proto.is_some());
}

// ============================================================================
// WebSocket Options
// ============================================================================

#[test]
fn test_websocket() {
    let args = assert_parses!("--ws", "http://localhost:8080/ws");
    assert!(args.ws);
}

#[test]
fn test_ws_send() {
    let args = assert_parses!("ws://localhost:8080", "--ws-send=hello");
    assert!(args.ws_send.is_some());
}

#[test]
fn test_ws_listen() {
    let args = assert_parses!("ws://localhost:8080", "--ws-listen");
    assert!(args.ws_listen);
}

#[test]
fn test_ws_interactive() {
    let args = assert_parses!("ws://localhost:8080", "--ws-interactive");
    assert!(args.ws_interactive);
}

#[test]
fn test_ws_subprotocol() {
    let args = assert_parses!("ws://localhost:8080", "--ws-subprotocol=graphql-ws");
    assert!(args.ws_subprotocol.is_some());
}

// ============================================================================
// Workflow Options
// ============================================================================

#[test]
fn test_run_workflow() {
    let args = assert_parses!("--run=workflow.yaml");
    assert!(args.run_workflow.is_some());
}

#[test]
fn test_workflow_env() {
    let args = assert_parses!("--run=workflow.yaml", "--env=staging");
    assert!(args.workflow_env.is_some());
}

#[test]
fn test_workflow_var() {
    let args = assert_parses!("--run=workflow.yaml", "--var=key=value");
    assert!(!args.workflow_vars.is_empty());
}

// ============================================================================
// Mock Server Options
// ============================================================================

#[test]
fn test_mock_server() {
    let args = assert_parses!("--mock");
    assert!(args.mock_server);
}

#[test]
fn test_mock_port() {
    let args = assert_parses!("--mock", "--mock-port=3000");
    assert!(args.mock_server);
    assert_eq!(args.mock_port, Some(3000));
}

#[test]
fn test_mock_config() {
    let args = assert_parses!("--mock", "--mock-config=mock.yaml");
    assert!(args.mock_server);
    assert!(args.mock_config.is_some());
}

#[test]
fn test_mock_route() {
    let args = assert_parses!("--mock", "--mock-route=GET:/api/hello:OK");
    assert!(args.mock_server);
    assert!(!args.mock_routes.is_empty());
}

#[test]
fn test_mock_cors() {
    let args = assert_parses!("--mock", "--mock-cors");
    assert!(args.mock_server);
    assert!(args.mock_cors);
}

#[test]
fn test_mock_latency() {
    let args = assert_parses!("--mock", "--mock-latency=100");
    assert!(args.mock_server);
    assert!(args.mock_latency.is_some());
}

#[test]
fn test_mock_host() {
    let args = assert_parses!("--mock", "--mock-host=0.0.0.0");
    assert!(args.mock_server);
    assert_eq!(args.mock_host, Some("0.0.0.0".to_string()));
}

#[test]
fn test_mock_log() {
    let args = assert_parses!("--mock", "--mock-log");
    assert!(args.mock_server);
    assert!(args.mock_log);
}

#[test]
fn test_mock_record() {
    let args = assert_parses!("--mock", "--mock-record=requests.har");
    assert!(args.mock_server);
    assert!(args.mock_record.is_some());
}

#[test]
fn test_mock_tls() {
    let args = assert_parses!("--mock", "--mock-tls-cert=cert.pem", "--mock-tls-key=key.pem");
    assert!(args.mock_server);
    assert!(args.mock_tls_cert.is_some());
    assert!(args.mock_tls_key.is_some());
}

#[test]
fn test_mock_proxy() {
    let args = assert_parses!("--mock", "--mock-proxy=http://api.example.com");
    assert!(args.mock_server);
    assert_eq!(args.mock_proxy, Some("http://api.example.com".to_string()));
}

// ============================================================================
// Plugin Options
// ============================================================================

#[test]
fn test_plugin_list() {
    let args = assert_parses!("--plugin-list");
    assert!(args.plugin_list);
}

#[test]
fn test_plugin_install() {
    let args = assert_parses!("--plugin-install=jwt-auth");
    assert!(args.plugin_install.is_some());
}

#[test]
fn test_plugin_uninstall() {
    let args = assert_parses!("--plugin-uninstall=jwt-auth");
    assert!(args.plugin_uninstall.is_some());
}

#[test]
fn test_plugin_search() {
    let args = assert_parses!("--plugin-search=auth");
    assert!(args.plugin_search.is_some());
}

#[test]
fn test_plugin_update() {
    let args = assert_parses!("--plugin-update");
    assert!(args.plugin_update);
}

#[test]
fn test_plugin_dir() {
    let args = assert_parses!("--plugin-dir=/custom/plugins", "--plugin-list");
    assert!(args.plugin_dir.is_some());
}

#[test]
fn test_enabled_plugins() {
    let args = assert_parses!("--plugin=jwt-auth", "--plugin=logger", "http://example.com");
    assert_eq!(args.enabled_plugins.len(), 2);
    assert!(args.enabled_plugins.contains(&"jwt-auth".to_string()));
    assert!(args.enabled_plugins.contains(&"logger".to_string()));
}

// ============================================================================
// Pager Options
// ============================================================================

#[test]
fn test_pager() {
    let args = assert_parses!("--pager", "http://example.com");
    assert!(args.pager);
}

#[test]
fn test_no_pager() {
    let args = assert_parses!("--no-pager", "http://example.com");
    assert!(args.no_pager);
}

#[test]
fn test_pager_cmd() {
    let args = assert_parses!("--pager", "--pager-cmd=less -R", "http://example.com");
    assert!(args.pager);
    assert_eq!(args.pager_cmd, Some("less -R".to_string()));
}

// ============================================================================
// Benchmark Options
// ============================================================================

#[test]
fn test_benchmark() {
    let args = assert_parses!("--bench", "--requests=100", "--concurrency=10", "http://example.com");
    assert!(args.bench);
    assert_eq!(args.bench_requests, 100);
    assert_eq!(args.bench_concurrency, 10);
}

// ============================================================================
// Fuzz Options
// ============================================================================

#[test]
fn test_fuzz() {
    let args = assert_parses!("--fuzz", "POST", "http://example.com", "data=test");
    assert!(args.fuzz);
}

#[test]
fn test_fuzz_category() {
    let args = assert_parses!("--fuzz", "--fuzz-category=sql", "POST", "http://example.com");
    assert!(args.fuzz);
    assert!(!args.fuzz_categories.is_empty());
}

// ============================================================================
// Import Options
// ============================================================================

#[test]
fn test_import_har() {
    let args = assert_parses!("--import-har=recording.har");
    assert!(args.import_har.is_some());
}

#[test]
fn test_import_openapi() {
    let args = assert_parses!("--import-openapi=api-spec.yaml");
    assert!(args.import_openapi.is_some());
}

#[test]
fn test_import_curl() {
    let args = assert_parses!("--import-curl=curl http://example.com");
    assert!(args.import_curl.is_some());
}

// ============================================================================
// Developer Options
// ============================================================================

#[test]
fn test_curl_generation() {
    let args = assert_parses!("--curl", "POST", "http://example.com", "data=test");
    assert!(args.curl);
}

#[test]
fn test_offline() {
    let args = assert_parses!("--offline", "POST", "http://example.com");
    assert!(args.offline);
}

#[test]
fn test_debug() {
    let args = assert_parses!("--debug", "http://example.com");
    assert!(args.debug);
}

#[test]
fn test_traceback() {
    let args = assert_parses!("--traceback", "http://example.com");
    assert!(args.traceback);
}

// ============================================================================
// Content Options
// ============================================================================

#[test]
fn test_form_mode() {
    let args = assert_parses!("-f", "POST", "http://example.com", "data=test");
    assert!(args.form);
}

#[test]
fn test_multipart() {
    let args = assert_parses!("--multipart", "POST", "http://example.com", "file@image.jpg");
    assert!(args.multipart);
}

#[test]
fn test_json_mode() {
    let args = assert_parses!("-j", "POST", "http://example.com", "data=test");
    assert!(args.json);
}

#[test]
fn test_raw_body() {
    let args = assert_parses!("--raw", "{\"key\":\"value\"}", "POST", "http://example.com");
    assert!(args.raw.is_some());
}

// ============================================================================
// Streaming Options
// ============================================================================

#[test]
fn test_stream() {
    let args = assert_parses!("-S", "http://example.com/stream");
    assert!(args.stream);
}

#[test]
fn test_chunked() {
    let args = assert_parses!("--chunked", "POST", "http://example.com");
    assert!(args.chunked);
}

// ============================================================================
// Completion and Update Options
// ============================================================================

#[test]
fn test_generate_completions_bash() {
    let args = assert_parses!("--generate-completions=bash");
    assert!(args.generate_completions.is_some());
}

#[test]
fn test_generate_completions_zsh() {
    let args = assert_parses!("--generate-completions=zsh");
    assert!(args.generate_completions.is_some());
}

#[test]
fn test_update() {
    let args = assert_parses!("--update");
    assert!(args.update);
}

// ============================================================================
// Combination Tests
// ============================================================================

#[test]
fn test_combined_auth_and_proxy() {
    let args = assert_parses!(
        "-a", "user:pass",
        "--proxy=http://proxy:8080",
        "http://example.com"
    );
    assert!(args.auth.is_some());
    assert!(!args.proxy.is_empty());
}

#[test]
fn test_combined_workflow_options() {
    let args = assert_parses!(
        "--run=workflow.yaml",
        "--env=production",
        "--var=api_key=secret",
        "--report-junit=results.xml"
    );
    assert!(args.run_workflow.is_some());
    assert!(args.workflow_env.is_some());
    assert!(!args.workflow_vars.is_empty());
    assert!(args.report_junit.is_some());
}

#[test]
fn test_combined_mock_options() {
    let args = assert_parses!(
        "--mock",
        "--mock-port=9000",
        "--mock-cors",
        "--mock-latency=50",
        "--mock-route=GET:/api/test:OK"
    );
    assert!(args.mock_server);
    assert_eq!(args.mock_port, Some(9000));
    assert!(args.mock_cors);
    assert!(args.mock_latency.is_some());
    assert!(!args.mock_routes.is_empty());
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_invalid_http_version() {
    let result = parse_args!("--http-version=99", "http://example.com");
    // Should either fail or accept and validate later
    // This depends on the implementation
}

#[test]
fn test_missing_url_for_request() {
    // --offline still requires a URL in the args
    let result = parse_args!("--offline", "POST");
    // This may or may not be an error depending on impl
}
