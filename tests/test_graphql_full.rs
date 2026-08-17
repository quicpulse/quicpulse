//! Unit tests for GraphQL request builder, variables, and introspection

use quicpulse::cli::Args;
use quicpulse::graphql::introspection::build_introspection_query;
use quicpulse::graphql::query::GraphQLRequestBuilder;
use quicpulse::graphql::{build_graphql_body, is_graphql_request};
use serde_json::json;

#[test]
fn test_is_graphql_request_detection() {
    let args_plain = Args::default();
    assert!(!is_graphql_request(&args_plain));

    let args_gql = Args {
        graphql: true,
        ..Default::default()
    };
    assert!(is_graphql_request(&args_gql));

    let args_query = Args {
        graphql_query: Some("query { me { id } }".to_string()),
        ..Default::default()
    };
    assert!(is_graphql_request(&args_query));

    let args_schema = Args {
        graphql_schema: true,
        ..Default::default()
    };
    assert!(is_graphql_request(&args_schema));
}

#[test]
fn test_graphql_request_builder() {
    let req = GraphQLRequestBuilder::new()
        .query("query GetUser($id: ID!) { user(id: $id) { name email } }")
        .operation_name("GetUser")
        .variable("id".to_string(), json!("12345"))
        .build()
        .unwrap();

    let json_val = req.to_json();
    assert_eq!(json_val["operationName"], "GetUser");
    assert!(json_val["query"]
        .as_str()
        .unwrap()
        .contains("user(id: $id)"));
    assert_eq!(json_val["variables"]["id"], "12345");
}

#[test]
fn test_build_graphql_body_from_args_and_data() {
    let args = Args {
        graphql_query: Some(
            "mutation AddTodo($text: String!) { addTodo(text: $text) { id } }".to_string(),
        ),
        graphql_operation: Some("AddTodo".to_string()),
        ..Default::default()
    };

    let data = json!({
        "text": "Buy milk"
    });

    let body = build_graphql_body(&args, &data).unwrap();
    assert_eq!(body["operationName"], "AddTodo");
    assert!(body["query"]
        .as_str()
        .unwrap()
        .contains("addTodo(text: $text)"));
    assert_eq!(body["variables"]["text"], "Buy milk");
}

#[test]
fn test_build_introspection_query() {
    let query = build_introspection_query();
    assert!(query.contains("__schema"));
    assert!(query.contains("queryType"));
    assert!(query.contains("mutationType"));
    assert!(query.contains("types"));
}
