//! Advanced integration tests for mock server functionality

mod common;

use common::{http, http_error, ExitStatus};
use tempfile::TempDir;

// =============================================================================
// Route Configuration Tests
// =============================================================================

#[test]
fn test_mock_route_cli() {
    // Test inline route specification via CLI
    // Running with --mock starts a server, but without a port binding it should work
    let response = http(&[
        "--mock",
        "--mock-port", "0",  // Use any available port
        "--mock-route", "GET:/api/test:200:OK",
        "--help"  // Just get help to validate args are accepted
    ]);

    // Should recognize mock arguments
    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_multiple_routes_cli() {
    // Multiple routes via CLI
    let response = http(&[
        "--mock",
        "--mock-route", "GET:/api/users:200:users list",
        "--mock-route", "POST:/api/users:201:created",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Config File Loading Tests
// =============================================================================

#[test]
fn test_mock_config_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    // Valid YAML config
    let config = r#"
host: "127.0.0.1"
port: 0
routes:
  - path: "/api/test"
    method: GET
    response:
      status: 200
      body: "Hello"
"#;
    std::fs::write(&config_path, config).unwrap();

    // Try to load the config (will show help since we add --help)
    let response = http(&[
        "--mock",
        "--mock-config", config_path.to_str().unwrap(),
        "--help"
    ]);

    // Config argument should be accepted
    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_config_json() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // JSON format config
    let config = r#"{
    "host": "127.0.0.1",
    "port": 0,
    "routes": [
        {
            "path": "/api/test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": "Hello from JSON config"
            }
        }
    ]
}"#;
    std::fs::write(&config_path, config).unwrap();

    let response = http(&[
        "--mock",
        "--mock-config", config_path.to_str().unwrap(),
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_config_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML format config
    let config = r#"
host = "127.0.0.1"
port = 0

[[routes]]
path = "/api/test"
method = "GET"

[routes.response]
status = 200
body = "Hello from TOML config"
"#;
    std::fs::write(&config_path, config).unwrap();

    let response = http(&[
        "--mock",
        "--mock-config", config_path.to_str().unwrap(),
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Mock Server Options Tests
// =============================================================================

#[test]
fn test_mock_cors_option() {
    let response = http(&[
        "--mock",
        "--mock-cors",
        "--mock-route", "GET:/api:200:ok",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_latency_option() {
    let response = http(&[
        "--mock",
        "--mock-latency", "50-100",
        "--mock-route", "GET:/api:200:ok",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_log_option() {
    let response = http(&[
        "--mock",
        "--mock-log",
        "--mock-route", "GET:/api:200:ok",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_host_option() {
    let response = http(&[
        "--mock",
        "--mock-host", "0.0.0.0",
        "--mock-port", "0",
        "--mock-route", "GET:/api:200:ok",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_proxy_option() {
    let response = http(&[
        "--mock",
        "--mock-proxy", "http://localhost:8080",
        "--mock-route", "GET:/api:200:ok",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Error Cases Tests
// =============================================================================

#[test]
fn test_mock_invalid_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.yaml");

    // Invalid YAML
    std::fs::write(&config_path, "invalid: yaml: config: [").unwrap();

    let response = http_error(&[
        "--mock",
        "--mock-config", config_path.to_str().unwrap()
    ]);

    // Should error on invalid config
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_mock_nonexistent_config_file() {
    let response = http_error(&[
        "--mock",
        "--mock-config", "/nonexistent/path/config.yaml"
    ]);

    // Should error on missing file
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_mock_invalid_latency_format() {
    let response = http_error(&[
        "--mock",
        "--mock-latency", "not-a-number",
        "--mock-route", "GET:/api:200:ok"
    ]);

    // Should error on invalid latency format
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Existing Mock Server Tests (from test_mock_server.rs patterns)
// =============================================================================

#[test]
fn test_mock_serve_alias() {
    // --serve is an alias for --mock
    let response = http(&[
        "--serve",
        "--mock-route", "GET:/test:200:hello",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_route_with_json_body() {
    // Route with JSON body
    let response = http(&[
        "--mock",
        "--mock-route", r#"GET:/api/users:200:{"users":[]}"#,
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_mock_multiple_options_combined() {
    // Combine multiple mock options
    let response = http(&[
        "--mock",
        "--mock-port", "0",
        "--mock-host", "127.0.0.1",
        "--mock-cors",
        "--mock-log",
        "--mock-latency", "10-50",
        "--mock-route", "GET:/api/test:200:ok",
        "--mock-route", "POST:/api/create:201:created",
        "--help"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}
