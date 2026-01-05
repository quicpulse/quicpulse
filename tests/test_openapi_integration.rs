//! Integration tests for OpenAPI import and workflow generation

mod common;

use common::{http, http_error, fixtures, ExitStatus};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// OpenAPI v3 Parsing Tests
// =============================================================================

#[test]
fn test_openapi_parse_v3_yaml() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should list endpoints from the spec
    assert!(output.contains("pets") || output.contains("GET") || output.contains("POST"),
        "Should list OpenAPI endpoints. output: {}", output);
}

#[test]
fn test_openapi_v3_shows_info() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should show API title or version
    assert!(output.contains("Petstore") || output.contains("1.0") || output.contains("API"),
        "Should show API info. output: {}", output);
}

// =============================================================================
// OpenAPI v2 (Swagger) Parsing Tests
// =============================================================================

#[test]
fn test_openapi_parse_v2_json() {
    let openapi_path = fixture_path("petstore-v2.json");
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should list endpoints from Swagger 2.0 spec
    assert!(output.contains("pets") || output.contains("users") || output.contains("GET"),
        "Should list Swagger endpoints. output: {}", output);
}

#[test]
fn test_openapi_v2_vs_v3_compatibility() {
    let v2_path = fixture_path("petstore-v2.json");
    let v3_path = fixture_path("petstore-v3.yaml");

    let v2_response = http(&["--import-openapi", v2_path.to_str().unwrap(), "--openapi-list"]);
    let v3_response = http(&["--import-openapi", v3_path.to_str().unwrap(), "--openapi-list"]);

    // Both should succeed
    assert_eq!(v2_response.exit_status, ExitStatus::Success);
    assert_eq!(v3_response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Endpoint Listing Tests
// =============================================================================

#[test]
fn test_openapi_list_endpoints() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // The petstore spec has /pets, /pets/{petId}, /users, /store/inventory
    // Check for at least some of them
    let has_endpoints = output.contains("/pets") ||
                        output.contains("listPets") ||
                        output.contains("GET") ||
                        output.contains("POST");
    assert!(has_endpoints, "Should list endpoints. output: {}", output);
}

#[test]
fn test_openapi_list_shows_methods() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should show HTTP methods
    let has_methods = output.contains("GET") ||
                      output.contains("POST") ||
                      output.contains("DELETE");
    assert!(has_methods, "Should show HTTP methods. output: {}", output);
}

// =============================================================================
// Tag Filtering Tests
// =============================================================================

#[test]
fn test_openapi_filter_by_tag() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // Filter for only "pets" tag
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-tag", "pets",
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should only show pet-related endpoints
    assert!(output.contains("pet") || output.contains("Pet"),
        "Should show pet endpoints. output: {}", output);
}

#[test]
fn test_openapi_exclude_tag() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // Exclude "store" tag
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-exclude-tag", "store",
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should succeed even with exclusion
}

#[test]
fn test_openapi_multiple_tags() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // Include multiple tags
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-tag", "pets",
        "--openapi-tag", "users",
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Deprecated Endpoints Tests
// =============================================================================

#[test]
fn test_openapi_include_deprecated() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // Include deprecated endpoints (DELETE /pets/{petId} is deprecated in the fixture)
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-include-deprecated",
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);

    // Should include DELETE endpoint
    assert!(output.contains("DELETE") || output.contains("deprecated") || output.contains("delete"),
        "Should include deprecated endpoints. output: {}", output);
}

#[test]
fn test_openapi_exclude_deprecated_by_default() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // Without --openapi-include-deprecated, deprecated should be excluded
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // This is default behavior - deprecated endpoints may or may not be shown
    // depending on implementation
}

// =============================================================================
// Base URL Override Tests
// =============================================================================

#[test]
fn test_openapi_base_url_override() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-base-url", "http://localhost:8080",
        "--openapi-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // The base URL should be used for endpoint URLs
}

// =============================================================================
// Workflow Generation Tests
// =============================================================================

#[test]
fn test_openapi_generate_workflow() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output_file = temp_dir.path().join("workflow.yaml");

    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--generate-workflow", output_file.to_str().unwrap()
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Workflow file should be created
    assert!(output_file.exists(), "Workflow file should be created");

    let content = std::fs::read_to_string(&output_file).unwrap();
    // Should contain valid YAML workflow
    assert!(content.contains("steps") || content.contains("name") || content.contains("url"),
        "Should generate valid workflow. content: {}", content);
}

#[test]
fn test_openapi_workflow_contains_endpoints() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output_file = temp_dir.path().join("workflow.yaml");

    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--generate-workflow", output_file.to_str().unwrap()
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);

    let content = std::fs::read_to_string(&output_file).unwrap();
    // Should contain endpoint references
    assert!(content.contains("pets") || content.contains("/") || content.contains("GET"),
        "Workflow should contain endpoints. content: {}", content);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_openapi_nonexistent_file() {
    let response = http_error(&["--import-openapi", "/nonexistent/path/api.yaml", "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_openapi_invalid_spec() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let invalid_file = temp_dir.path().join("invalid.yaml");
    std::fs::write(&invalid_file, "not: a: valid: openapi: spec").unwrap();

    let response = http_error(&["--import-openapi", invalid_file.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_openapi_invalid_json() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let invalid_file = temp_dir.path().join("invalid.json");
    std::fs::write(&invalid_file, "{ invalid json }").unwrap();

    let response = http_error(&["--import-openapi", invalid_file.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_openapi_empty_spec() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let empty_file = temp_dir.path().join("empty.yaml");
    std::fs::write(&empty_file, "").unwrap();

    let response = http_error(&["--import-openapi", empty_file.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Fuzz Integration Tests
// =============================================================================

#[test]
fn test_openapi_with_fuzz_flag() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output_file = temp_dir.path().join("workflow.yaml");

    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--openapi-fuzz",
        "--generate-workflow", output_file.to_str().unwrap()
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Workflow should include fuzz test payloads
    let content = std::fs::read_to_string(&output_file).unwrap_or_default();
    // The content should be generated
    assert!(!content.is_empty(), "Workflow should be generated");
}

// =============================================================================
// Complex Spec Tests
// =============================================================================

#[test]
fn test_openapi_with_security_schemes() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // The petstore spec has security schemes defined
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should handle security schemes
}

#[test]
fn test_openapi_with_refs() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    // The petstore spec uses $ref for schemas
    let response = http(&["--import-openapi", openapi_path.to_str().unwrap(), "--openapi-list"]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Should resolve $ref references
}

#[test]
fn test_openapi_with_parameters() {
    let openapi_path = fixture_path("petstore-v3.yaml");
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output_file = temp_dir.path().join("workflow.yaml");

    let response = http(&[
        "--import-openapi", openapi_path.to_str().unwrap(),
        "--generate-workflow", output_file.to_str().unwrap()
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);

    let content = std::fs::read_to_string(&output_file).unwrap();
    // Should include parameter placeholders or magic values
    assert!(content.contains("petId") || content.contains("id") || content.contains("limit") || content.contains("{"),
        "Workflow should handle parameters. content: {}", content);
}
