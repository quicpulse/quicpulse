//! GraphQL support module
//!
//! This module provides GraphQL query building, variable handling,
//! and schema introspection capabilities.

pub mod query;
pub mod variables;
pub mod introspection;

pub use query::GraphQLRequestBuilder;
pub use introspection::build_introspection_query;

use crate::cli::Args;
use crate::errors::QuicpulseError;
use serde_json::{json, Value as JsonValue};

/// Check if request should be treated as GraphQL
pub fn is_graphql_request(args: &Args) -> bool {
    args.graphql || args.graphql_query.is_some() || args.graphql_schema
}

/// Build a GraphQL request body from CLI arguments
pub fn build_graphql_body(args: &Args, data: &JsonValue) -> Result<JsonValue, QuicpulseError> {
    let mut builder = GraphQLRequestBuilder::new();

    // Get query from --graphql-query or from data's "query" field
    if let Some(ref query) = args.graphql_query {
        builder = builder.query(query);
    } else if let Some(query_val) = data.get("query") {
        if let Some(query_str) = query_val.as_str() {
            builder = builder.query(query_str);
        }
    }

    // Add operation name if specified
    if let Some(ref op_name) = args.graphql_operation {
        builder = builder.operation_name(op_name);
    } else if let Some(op_val) = data.get("operationName") {
        if let Some(op_str) = op_val.as_str() {
            builder = builder.operation_name(op_str);
        }
    }

    // Add variables from data
    if let Some(vars) = data.get("variables") {
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                builder = builder.variable(k.clone(), v.clone());
            }
        }
    }

    // Add any other data fields as variables (excluding query, operationName, variables)
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            if k != "query" && k != "operationName" && k != "variables" {
                builder = builder.variable(k.clone(), v.clone());
            }
        }
    }

    let request = builder.build()?;
    Ok(request.to_json())
}

/// Build an introspection query request
pub fn build_schema_request() -> JsonValue {
    let query = build_introspection_query();
    json!({
        "query": query,
        "operationName": "IntrospectionQuery"
    })
}

/// Format a GraphQL response for display
pub fn format_graphql_response(response: &JsonValue, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(response).unwrap_or_else(|_| response.to_string())
    } else {
        response.to_string()
    }
}

/// Extract errors from GraphQL response
pub fn extract_errors(response: &JsonValue) -> Option<Vec<String>> {
    response.get("errors").and_then(|errors| {
        errors.as_array().map(|arr| {
            arr.iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .map(String::from)
                .collect()
        })
    })
}

/// Check if GraphQL response has errors
pub fn has_errors(response: &JsonValue) -> bool {
    response.get("errors").map(|e| !e.is_null() && e.as_array().map(|a| !a.is_empty()).unwrap_or(false)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_graphql_body_with_query() {
        let mut args = Args::default();
        args.graphql = true;
        args.graphql_query = Some("{ users { id name } }".to_string());

        let data = json!({});
        let result = build_graphql_body(&args, &data).unwrap();

        assert_eq!(result["query"], "{ users { id name } }");
    }

    #[test]
    fn test_extract_errors() {
        let response = json!({
            "errors": [
                {"message": "Field 'foo' not found"},
                {"message": "Invalid query"}
            ]
        });

        let errors = extract_errors(&response).unwrap();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "Field 'foo' not found");
    }

    #[test]
    fn test_has_errors() {
        let with_errors = json!({"errors": [{"message": "Error"}]});
        let without_errors = json!({"data": {}});

        assert!(has_errors(&with_errors));
        assert!(!has_errors(&without_errors));
    }
}
