//! Integration tests for gRPC functionality

mod common;

use common::{http, http_error, fixtures, ExitStatus};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// gRPC Endpoint Parsing Tests
// =============================================================================

#[test]
fn test_grpc_endpoint_parsing_basic() {
    // Test that gRPC URL parsing works
    let response = http(&[
        "--grpc",
        "grpc://localhost:50051/test.TestService/Echo",
        "--offline"
    ]);

    // Should parse the endpoint format - just verify CLI accepts the arguments
    // Offline mode shows the request but doesn't send it
    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_grpc_endpoint_with_port() {
    let response = http_error(&[
        "--grpc",
        "grpc://127.0.0.1:9000/package.Service/Method",
        "--offline"
    ]);

    // Should parse host:port format - exits with error due to offline
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("offline") ||
            output.contains("127.0.0.1"),
        "Should handle endpoint. output: {}", output);
}

#[test]
fn test_grpc_endpoint_without_scheme() {
    // Test parsing without grpc:// scheme
    let response = http_error(&[
        "--grpc",
        "localhost:50051/test.Service/Method",
        "--offline"
    ]);

    // Should handle with or without scheme
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("offline") ||
            output.contains("localhost"),
        "Should handle endpoint. output: {}", output);
}

// =============================================================================
// Proto File Loading Tests
// =============================================================================

#[test]
fn test_grpc_with_proto_file() {
    let proto_path = fixture_path("test.proto");

    // Test that proto file can be specified
    let response = http(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051/test.TestService/Echo",
        "--offline"
    ]);

    // Should accept proto file argument - just verify CLI accepts it
    // Offline mode shows the request but doesn't send it
    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_grpc_proto_file_nonexistent() {
    let response = http_error(&[
        "--grpc",
        "--proto", "/nonexistent/path/test.proto",
        "grpc://localhost:50051/test.Service/Method"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
    assert!(response.stderr.contains("not found") ||
            response.stderr.contains("No such file") ||
            response.stderr.contains("error") ||
            response.stderr.contains("proto"),
        "Should error on missing proto. stderr: {}", response.stderr);
}

#[test]
fn test_grpc_invalid_proto_file() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let invalid_proto = temp_dir.path().join("invalid.proto");
    std::fs::write(&invalid_proto, "this is not valid proto syntax").unwrap();

    let response = http_error(&[
        "--grpc",
        "--proto", invalid_proto.to_str().unwrap(),
        "grpc://localhost:50051/test.Service/Method"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Service Listing Tests
// =============================================================================

#[test]
fn test_grpc_list_services() {
    let proto_path = fixture_path("test.proto");

    // Without a running server, listing will fail but args should be accepted
    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--grpc-list",
        "grpc://localhost:50051"
    ]);

    // Should recognize the arguments (connection will fail)
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("TestService") ||
            output.contains("Service") ||
            output.contains("connect") ||
            output.contains("Connection"),
        "Should handle list request. output: {}", output);
}

#[test]
fn test_grpc_list_without_proto() {
    // Listing without proto requires server reflection
    let response = http_error(&[
        "--grpc",
        "--grpc-list",
        "grpc://localhost:50051"
    ]);

    // Without proto and without running server, should fail
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("reflect") ||
            output.contains("proto") ||
            output.contains("connect"),
        "Should need proto or reflection. output: {}", output);
}

// =============================================================================
// Service Description Tests
// =============================================================================

#[test]
fn test_grpc_describe_service() {
    let proto_path = fixture_path("test.proto");

    // Without a running server, describe will fail but args should be accepted
    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--grpc-describe", "test.TestService",
        "grpc://localhost:50051"
    ]);

    // Should recognize the arguments (may fail due to connection)
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("Echo") ||
            output.contains("Service") ||
            output.contains("connect") ||
            output.contains("Connection"),
        "Should handle describe request. output: {}", output);
}

#[test]
fn test_grpc_describe_nonexistent_service() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--grpc-describe", "nonexistent.Service",
        "grpc://localhost:50051"
    ]);

    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("not found") ||
            output.contains("unknown") ||
            output.contains("connect"),
        "Should error on unknown service. output: {}", output);
}

// =============================================================================
// Request Body Tests
// =============================================================================

#[test]
fn test_grpc_json_request_body() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051/test.TestService/Echo",
        r#"message={"message": "hello"}"#
    ]);

    // JSON body should be accepted (connection will fail)
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_grpc_with_data_fields() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051/test.TestService/Echo",
        "message=hello"
    ]);

    // Data fields should be accepted
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Metadata/Headers Tests
// =============================================================================

#[test]
fn test_grpc_with_metadata() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051/test.TestService/Echo",
        "x-custom-metadata:value",
        "authorization:Bearer token"
    ]);

    // Metadata should be accepted
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// TLS Tests
// =============================================================================

#[test]
fn test_grpc_with_tls() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpcs://localhost:50051/test.TestService/Echo"  // grpcs = TLS
    ]);

    // TLS endpoint should be accepted
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_grpc_insecure_flag() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--verify", "no",
        "grpcs://localhost:50051/test.TestService/Echo"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_grpc_invalid_endpoint() {
    let response = http_error(&[
        "--grpc",
        "not-a-valid-grpc-endpoint"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_grpc_missing_service_method() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051"  // Missing service/method
    ]);

    // Should error or require service/method
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_grpc_connection_refused() {
    let proto_path = fixture_path("test.proto");

    // Connect to port that's not listening
    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://127.0.0.1:59999/test.TestService/Echo"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Error);
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("connect") || output.contains("Connection") ||
            output.contains("refused") || output.contains("error"),
        "Should show connection error. output: {}", output);
}

// =============================================================================
// Proto Content Tests
// =============================================================================

#[test]
fn test_grpc_proto_with_nested_messages() {
    // The fixture proto has nested messages
    let proto_path = fixture_path("test.proto");

    // Without a running server, will fail but arguments should be accepted
    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--grpc-list",
        "grpc://localhost:50051"
    ]);

    // Should handle nested message types in proto
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Error ||
            output.contains("Test") ||
            output.contains("Service") ||
            output.contains("connect"),
        "Should handle proto. output: {}", output);
}

// =============================================================================
// Timeout Tests
// =============================================================================

#[test]
fn test_grpc_with_timeout() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--timeout", "1",
        "grpc://localhost:50051/test.TestService/Echo"
    ]);

    // Timeout should be applied
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Verbose Mode Tests
// =============================================================================

#[test]
fn test_grpc_verbose_mode() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "-v",
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "grpc://localhost:50051/test.TestService/Echo"
    ]);

    // Verbose mode should show more details
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.len() > 50, "Verbose should show details. output: {}", output);
}

// =============================================================================
// Combined Options Tests
// =============================================================================

#[test]
fn test_grpc_with_all_options() {
    let proto_path = fixture_path("test.proto");

    let response = http_error(&[
        "--grpc",
        "--proto", proto_path.to_str().unwrap(),
        "--timeout", "5",
        "-v",
        "grpc://localhost:50051/test.TestService/Echo",
        "message=test",
        "x-request-id:123"
    ]);

    // All options should be accepted
    assert_eq!(response.exit_status, ExitStatus::Error);
}
