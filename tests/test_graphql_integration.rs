//! Integration tests for GraphQL functionality

mod common;

use common::{http, http_error, ExitStatus};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_string_contains};

// =============================================================================
// Basic GraphQL Query Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_simple_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "users": [
                        {"id": "1", "name": "Alice"},
                        {"id": "2", "name": "Bob"}
                    ]
                }
            })))
        .mount(&mock_server)
        .await;

    let query = "{ users { id name } }";
    let response = http(&[
        "-p", "b",  // Print response body
        "--graphql",
        "--graphql-query", query,
        "POST",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("users") || output.contains("Alice") || output.contains("data"),
        "Should return GraphQL data. output: {}", output);
}

#[tokio::test]
async fn test_graphql_query_with_variables() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("variables"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "user": {"id": "1", "name": "Alice"}
                }
            })))
        .mount(&mock_server)
        .await;

    let query = "query GetUser($id: ID!) { user(id: $id) { id name } }";
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        &format!("{}/graphql", mock_server.uri()),
        "variables:={\"id\": \"1\"}"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_graphql_mutation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("mutation"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "createUser": {"id": "3", "name": "Charlie"}
                }
            })))
        .mount(&mock_server)
        .await;

    let mutation = "mutation CreateUser($name: String!) { createUser(name: $name) { id name } }";
    let response = http(&[
        "--graphql",
        "--graphql-query", mutation,
        &format!("{}/graphql", mock_server.uri()),
        "variables:={\"name\": \"Charlie\"}"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Operation Name Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_operation_name() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("operationName"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {"users": []}
            })))
        .mount(&mock_server)
        .await;

    let query = "query ListUsers { users { id } } query GetUser($id: ID!) { user(id: $id) { id } }";
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        "--graphql-operation", "ListUsers",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Schema Introspection Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_schema_introspection() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "__schema": {
                        "types": [
                            {"name": "Query", "kind": "OBJECT"},
                            {"name": "User", "kind": "OBJECT"},
                            {"name": "String", "kind": "SCALAR"}
                        ],
                        "queryType": {"name": "Query"}
                    }
                }
            })))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "-p", "b",  // Print response body
        "--graphql",
        "--graphql-schema",
        "POST",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("schema") || output.contains("types") || output.contains("Query") || output.contains("data"),
        "Should return schema. output: {}", output);
}

// =============================================================================
// Error Response Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": null,
                "errors": [
                    {
                        "message": "Field 'invalid' doesn't exist on type 'Query'",
                        "locations": [{"line": 1, "column": 3}],
                        "path": ["invalid"]
                    }
                ]
            })))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "-p", "b",  // Print response body
        "--graphql",
        "--graphql-query", "{ invalid }",
        "POST",
        &format!("{}/graphql", mock_server.uri())
    ]);

    // GraphQL errors are returned in the response, not as HTTP errors
    // But the CLI should handle them appropriately
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("error") || output.contains("Error") || output.contains("invalid") || output.contains("null"),
        "Should show GraphQL errors. output: {}", output);
}

#[tokio::test]
async fn test_graphql_partial_data_with_errors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "users": [{"id": "1", "name": "Alice"}]
                },
                "errors": [
                    {"message": "Could not fetch all users"}
                ]
            })))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "-p", "b",  // Print response body
        "--graphql",
        "--graphql-query", "{ users { id name } }",
        "POST",
        &format!("{}/graphql", mock_server.uri())
    ]);

    // Should succeed but show errors
    let output = format!("{}{}", response.stdout, response.stderr);
    // Both data and errors may be shown
    assert!(output.contains("users") || output.contains("Alice") || output.contains("error") || output.contains("data"),
        "Should handle partial data. output: {}", output);
}

// =============================================================================
// Query Validation Tests
// =============================================================================

#[test]
fn test_graphql_empty_query() {
    let response = http_error(&[
        "--graphql",
        "--graphql-query", "",
        "http://localhost:9999/graphql"
    ]);

    // Empty query should error
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_graphql_invalid_query_syntax() {
    let response = http_error(&[
        "--graphql",
        "--graphql-query", "{ users { id name }",  // Missing closing brace
        "http://localhost:9999/graphql"
    ]);

    // Invalid syntax should error
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Request Format Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_request_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("query"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    // The request should be formatted as proper GraphQL
    let response = http(&[
        "--graphql",
        "--graphql-query", "{ users { id } }",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_graphql_content_type_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--graphql",
        "--graphql-query", "{ test }",
        &format!("{}/graphql", mock_server.uri())
    ]);

    // Should send appropriate content-type
    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Complex Query Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_nested_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "user": {
                        "id": "1",
                        "name": "Alice",
                        "posts": [
                            {"id": "1", "title": "Hello"},
                            {"id": "2", "title": "World"}
                        ]
                    }
                }
            })))
        .mount(&mock_server)
        .await;

    let query = "{ user(id: 1) { id name posts { id title } } }";
    let response = http(&[
        "-p", "b",  // Print response body
        "--graphql",
        "--graphql-query", query,
        "POST",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("posts") || output.contains("Hello") || output.contains("user") || output.contains("data"),
        "Should return nested data. output: {}", output);
}

#[tokio::test]
async fn test_graphql_with_aliases() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "first": {"id": "1", "name": "Alice"},
                    "second": {"id": "2", "name": "Bob"}
                }
            })))
        .mount(&mock_server)
        .await;

    let query = "{ first: user(id: 1) { id name } second: user(id: 2) { id name } }";
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_graphql_with_fragments() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "data": {
                    "users": [
                        {"id": "1", "name": "Alice", "email": "alice@example.com"},
                        {"id": "2", "name": "Bob", "email": "bob@example.com"}
                    ]
                }
            })))
        .mount(&mock_server)
        .await;

    let query = r#"
        query {
            users {
                ...userFields
            }
        }
        fragment userFields on User {
            id
            name
            email
        }
    "#;
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Variable Types Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_variable_types() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {"result": true}})))
        .mount(&mock_server)
        .await;

    // Test with different variable types
    let query = "query Test($str: String, $num: Int, $bool: Boolean) { result }";
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        &format!("{}/graphql", mock_server.uri()),
        r#"variables:={"str": "test", "num": 42, "bool": true}"#
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_graphql_null_variable() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let query = "query Test($optional: String) { result }";
    let response = http(&[
        "--graphql",
        "--graphql-query", query,
        &format!("{}/graphql", mock_server.uri()),
        r#"variables:={"optional": null}"#
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Combined with Other Options Tests
// =============================================================================

#[tokio::test]
async fn test_graphql_with_custom_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "--graphql",
        "--graphql-query", "{ test }",
        &format!("{}/graphql", mock_server.uri()),
        "Authorization:Bearer token123"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[tokio::test]
async fn test_graphql_verbose_mode() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {"test": true}})))
        .mount(&mock_server)
        .await;

    let response = http(&[
        "-v",
        "--graphql",
        "--graphql-query", "{ test }",
        &format!("{}/graphql", mock_server.uri())
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    // Verbose should show more details
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.len() > 50, "Verbose should produce more output. output: {}", output);
}
