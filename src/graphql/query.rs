//! GraphQL query building
//!
//! This module provides types for constructing GraphQL requests.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use crate::errors::QuicpulseError;

/// A GraphQL request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLRequest {
    /// The GraphQL query string
    pub query: String,

    /// Optional operation name (for documents with multiple operations)
    #[serde(rename = "operationName", skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,

    /// Optional variables for the query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<JsonValue>,
}

impl GraphQLRequest {
    /// Create a new GraphQL request with just a query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            operation_name: None,
            variables: None,
        }
    }

    /// Create a new request with query and variables
    pub fn with_variables(query: impl Into<String>, variables: JsonValue) -> Self {
        Self {
            query: query.into(),
            operation_name: None,
            variables: Some(variables),
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> JsonValue {
        let mut obj = json!({
            "query": self.query
        });

        if let Some(ref op_name) = self.operation_name {
            obj["operationName"] = json!(op_name);
        }

        if let Some(ref vars) = self.variables {
            if !vars.is_null() && vars.as_object().map(|o| !o.is_empty()).unwrap_or(true) {
                obj["variables"] = vars.clone();
            }
        }

        obj
    }

    /// Convert to JSON string
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.to_json()).unwrap_or_default()
    }

    /// Convert to pretty JSON string
    pub fn to_json_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).unwrap_or_default()
    }
}

/// Builder for GraphQL requests
#[derive(Debug, Default)]
pub struct GraphQLRequestBuilder {
    query: Option<String>,
    operation_name: Option<String>,
    variables: serde_json::Map<String, JsonValue>,
}

impl GraphQLRequestBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the query
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Set the operation name
    pub fn operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    /// Add a variable
    pub fn variable(mut self, name: impl Into<String>, value: JsonValue) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Add multiple variables from an object
    pub fn variables(mut self, vars: JsonValue) -> Self {
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                self.variables.insert(k.clone(), v.clone());
            }
        }
        self
    }

    /// Build the GraphQL request
    pub fn build(self) -> Result<GraphQLRequest, QuicpulseError> {
        let query = self.query.ok_or_else(|| {
            QuicpulseError::Argument("GraphQL query is required".to_string())
        })?;

        let variables = if self.variables.is_empty() {
            None
        } else {
            Some(JsonValue::Object(self.variables))
        };

        Ok(GraphQLRequest {
            query,
            operation_name: self.operation_name,
            variables,
        })
    }
}

/// Parse a GraphQL query to extract operation names
pub fn extract_operation_names(query: &str) -> Vec<String> {
    let mut names = Vec::new();

    // Simple regex-free parsing for operation names
    // Looks for patterns like "query Name" or "mutation Name" or "subscription Name"
    let query = query.trim();

    for line in query.lines() {
        let line = line.trim();
        for keyword in &["query", "mutation", "subscription"] {
            if let Some(rest) = line.strip_prefix(keyword) {
                let rest = rest.trim();
                // Extract the operation name (word before '(' or '{')
                if !rest.is_empty() && !rest.starts_with('(') && !rest.starts_with('{') {
                    if let Some(name) = rest.split(|c| c == '(' || c == '{' || c == ' ')
                        .next()
                        .filter(|s| !s.is_empty())
                    {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }

    names
}

/// Validate a GraphQL query (basic syntax check)
pub fn validate_query(query: &str) -> Result<(), QuicpulseError> {
    let query = query.trim();

    if query.is_empty() {
        return Err(QuicpulseError::Argument("Query cannot be empty".to_string()));
    }

    // Check for balanced braces
    let mut brace_count = 0i32;
    for c in query.chars() {
        match c {
            '{' => brace_count += 1,
            '}' => brace_count -= 1,
            _ => {}
        }
        if brace_count < 0 {
            return Err(QuicpulseError::Argument("Unbalanced braces in query".to_string()));
        }
    }

    if brace_count != 0 {
        return Err(QuicpulseError::Argument("Unbalanced braces in query".to_string()));
    }

    // Must have at least one selection set
    if !query.contains('{') {
        return Err(QuicpulseError::Argument("Query must contain a selection set".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_request_new() {
        let req = GraphQLRequest::new("{ users { id } }");
        assert_eq!(req.query, "{ users { id } }");
        assert!(req.operation_name.is_none());
        assert!(req.variables.is_none());
    }

    #[test]
    fn test_graphql_request_with_variables() {
        let vars = json!({"id": 123});
        let req = GraphQLRequest::with_variables("query($id: Int!) { user(id: $id) { name } }", vars);

        assert!(req.variables.is_some());
        assert_eq!(req.variables.as_ref().unwrap()["id"], 123);
    }

    #[test]
    fn test_builder() {
        let req = GraphQLRequestBuilder::new()
            .query("{ users { id } }")
            .operation_name("GetUsers")
            .variable("limit", json!(10))
            .build()
            .unwrap();

        assert_eq!(req.query, "{ users { id } }");
        assert_eq!(req.operation_name, Some("GetUsers".to_string()));
        assert!(req.variables.is_some());
    }

    #[test]
    fn test_extract_operation_names() {
        let query = r#"
            query GetUser($id: ID!) {
                user(id: $id) { name }
            }
            mutation UpdateUser($input: UserInput!) {
                updateUser(input: $input) { id }
            }
        "#;

        let names = extract_operation_names(query);
        assert!(names.contains(&"GetUser".to_string()));
        assert!(names.contains(&"UpdateUser".to_string()));
    }

    #[test]
    fn test_validate_query() {
        assert!(validate_query("{ users { id } }").is_ok());
        assert!(validate_query("query { users { id } }").is_ok());
        assert!(validate_query("").is_err());
        assert!(validate_query("users").is_err()); // no braces
        assert!(validate_query("{ users { id }").is_err()); // unbalanced
    }

    #[test]
    fn test_to_json() {
        let req = GraphQLRequestBuilder::new()
            .query("{ users { id } }")
            .operation_name("GetUsers")
            .build()
            .unwrap();

        let json = req.to_json();
        assert_eq!(json["query"], "{ users { id } }");
        assert_eq!(json["operationName"], "GetUsers");
    }
}
