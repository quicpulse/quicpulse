//! GraphQL schema introspection
//!
//! This module provides support for GraphQL schema introspection,
//! allowing discovery of types, fields, and operations.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Standard GraphQL introspection query
pub const INTROSPECTION_QUERY: &str = r#"
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      ...FullType
    }
    directives {
      name
      description
      locations
      args {
        ...InputValue
      }
    }
  }
}

fragment FullType on __Type {
  kind
  name
  description
  fields(includeDeprecated: true) {
    name
    description
    args {
      ...InputValue
    }
    type {
      ...TypeRef
    }
    isDeprecated
    deprecationReason
  }
  inputFields {
    ...InputValue
  }
  interfaces {
    ...TypeRef
  }
  enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
  }
  possibleTypes {
    ...TypeRef
  }
}

fragment InputValue on __InputValue {
  name
  description
  type {
    ...TypeRef
  }
  defaultValue
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
            }
          }
        }
      }
    }
  }
}
"#;

/// Build the standard introspection query
pub fn build_introspection_query() -> &'static str {
    INTROSPECTION_QUERY
}

/// Simplified introspection query (for basic schema info)
pub const SIMPLE_INTROSPECTION_QUERY: &str = r#"
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      kind
      name
      description
      fields {
        name
        description
        type {
          kind
          name
          ofType {
            kind
            name
          }
        }
      }
    }
  }
}
"#;

/// GraphQL introspection query builder
#[derive(Debug, Default)]
pub struct IntrospectionQuery {
    include_deprecated: bool,
    full_type_info: bool,
}

impl IntrospectionQuery {
    /// Create a new introspection query builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Include deprecated fields and enum values
    pub fn include_deprecated(mut self, include: bool) -> Self {
        self.include_deprecated = include;
        self
    }

    /// Include full type information (all nested types)
    pub fn full_type_info(mut self, full: bool) -> Self {
        self.full_type_info = full;
        self
    }

    /// Build the query string
    pub fn build(&self) -> &'static str {
        if self.full_type_info {
            INTROSPECTION_QUERY
        } else {
            SIMPLE_INTROSPECTION_QUERY
        }
    }
}

/// Parsed schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: Vec<TypeInfo>,
}

impl SchemaInfo {
    /// Parse from introspection response
    /// Returns Result with descriptive error instead of silently failing
    pub fn from_response(response: &JsonValue) -> Result<Self, String> {
        // Check for GraphQL errors first
        if let Some(errors) = response.get("errors") {
            if let Some(errors_arr) = errors.as_array() {
                if !errors_arr.is_empty() {
                    let error_messages: Vec<String> = errors_arr
                        .iter()
                        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                        .map(String::from)
                        .collect();
                    if !error_messages.is_empty() {
                        return Err(format!("GraphQL errors: {}", error_messages.join("; ")));
                    }
                }
            }
        }

        // Check for data field
        let data = response.get("data")
            .ok_or_else(|| "Response missing 'data' field - introspection may be disabled".to_string())?;

        // Handle null data (can happen when introspection is disabled)
        if data.is_null() {
            return Err("Introspection returned null data - introspection may be disabled on this server".to_string());
        }

        let schema = data.get("__schema")
            .ok_or_else(|| "Response missing '__schema' field - not a valid introspection response".to_string())?;

        let query_type = schema.get("queryType")
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

        let mutation_type = schema.get("mutationType")
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

        let subscription_type = schema.get("subscriptionType")
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

        let types = schema.get("types")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(TypeInfo::from_json)
                    .filter(|t| !t.name.starts_with("__")) // Filter introspection types
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            query_type,
            mutation_type,
            subscription_type,
            types,
        })
    }

    /// Get all query fields
    pub fn query_fields(&self) -> Vec<&FieldInfo> {
        self.query_type.as_ref()
            .and_then(|name| self.types.iter().find(|t| &t.name == name))
            .map(|t| t.fields.iter().collect())
            .unwrap_or_default()
    }

    /// Get all mutation fields
    pub fn mutation_fields(&self) -> Vec<&FieldInfo> {
        self.mutation_type.as_ref()
            .and_then(|name| self.types.iter().find(|t| &t.name == name))
            .map(|t| t.fields.iter().collect())
            .unwrap_or_default()
    }

    /// Find a type by name
    pub fn find_type(&self, name: &str) -> Option<&TypeInfo> {
        self.types.iter().find(|t| t.name == name)
    }

    /// Format schema for display
    pub fn format_display(&self) -> String {
        let mut output = String::new();

        output.push_str("Schema:\n");
        if let Some(ref qt) = self.query_type {
            output.push_str(&format!("  Query: {}\n", qt));
        }
        if let Some(ref mt) = self.mutation_type {
            output.push_str(&format!("  Mutation: {}\n", mt));
        }
        if let Some(ref st) = self.subscription_type {
            output.push_str(&format!("  Subscription: {}\n", st));
        }

        output.push_str("\nTypes:\n");
        for typ in &self.types {
            if typ.kind == "OBJECT" || typ.kind == "INPUT_OBJECT" {
                output.push_str(&format!("  {} ({}):\n", typ.name, typ.kind));
                for field in &typ.fields {
                    output.push_str(&format!("    {}: {}\n", field.name, field.type_name));
                }
            }
        }

        output
    }
}

/// Type information from introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldInfo>,
    pub input_fields: Vec<InputFieldInfo>,
    pub enum_values: Vec<EnumValueInfo>,
    pub interfaces: Vec<String>,
    pub possible_types: Vec<String>,
}

impl TypeInfo {
    /// Parse from JSON value
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let kind = value.get("kind")?.as_str()?.to_string();
        let name = value.get("name")?.as_str()?.to_string();

        let description = value.get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let fields = value.get("fields")
            .and_then(|f| f.as_array())
            .map(|arr| arr.iter().filter_map(FieldInfo::from_json).collect())
            .unwrap_or_default();

        let input_fields = value.get("inputFields")
            .and_then(|f| f.as_array())
            .map(|arr| arr.iter().filter_map(InputFieldInfo::from_json).collect())
            .unwrap_or_default();

        let enum_values = value.get("enumValues")
            .and_then(|e| e.as_array())
            .map(|arr| arr.iter().filter_map(EnumValueInfo::from_json).collect())
            .unwrap_or_default();

        let interfaces = value.get("interfaces")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let possible_types = value.get("possibleTypes")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            kind,
            name,
            description,
            fields,
            input_fields,
            enum_values,
            interfaces,
            possible_types,
        })
    }
}

/// Field information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub description: Option<String>,
    pub type_name: String,
    pub args: Vec<InputFieldInfo>,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

impl FieldInfo {
    /// Parse from JSON value
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let name = value.get("name")?.as_str()?.to_string();

        let description = value.get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let type_name = value.get("type")
            .map(format_type_ref)
            .unwrap_or_else(|| "Unknown".to_string());

        let args = value.get("args")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(InputFieldInfo::from_json).collect())
            .unwrap_or_default();

        let is_deprecated = value.get("isDeprecated")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        let deprecation_reason = value.get("deprecationReason")
            .and_then(|r| r.as_str())
            .map(String::from);

        Some(Self {
            name,
            description,
            type_name,
            args,
            is_deprecated,
            deprecation_reason,
        })
    }
}

/// Input field/argument information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFieldInfo {
    pub name: String,
    pub description: Option<String>,
    pub type_name: String,
    pub default_value: Option<String>,
}

impl InputFieldInfo {
    /// Parse from JSON value
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let name = value.get("name")?.as_str()?.to_string();

        let description = value.get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let type_name = value.get("type")
            .map(format_type_ref)
            .unwrap_or_else(|| "Unknown".to_string());

        let default_value = value.get("defaultValue")
            .and_then(|d| d.as_str())
            .map(String::from);

        Some(Self {
            name,
            description,
            type_name,
            default_value,
        })
    }
}

/// Enum value information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValueInfo {
    pub name: String,
    pub description: Option<String>,
    pub is_deprecated: bool,
    pub deprecation_reason: Option<String>,
}

impl EnumValueInfo {
    /// Parse from JSON value
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let name = value.get("name")?.as_str()?.to_string();

        let description = value.get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let is_deprecated = value.get("isDeprecated")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        let deprecation_reason = value.get("deprecationReason")
            .and_then(|r| r.as_str())
            .map(String::from);

        Some(Self {
            name,
            description,
            is_deprecated,
            deprecation_reason,
        })
    }
}

/// Format a type reference from introspection response
fn format_type_ref(type_ref: &JsonValue) -> String {
    let kind = type_ref.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    let name = type_ref.get("name").and_then(|n| n.as_str());

    match kind {
        "NON_NULL" => {
            let inner = type_ref.get("ofType")
                .map(format_type_ref)
                .unwrap_or_else(|| "Unknown".to_string());
            format!("{}!", inner)
        }
        "LIST" => {
            let inner = type_ref.get("ofType")
                .map(format_type_ref)
                .unwrap_or_else(|| "Unknown".to_string());
            format!("[{}]", inner)
        }
        _ => name.unwrap_or("Unknown").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_introspection_query_builder() {
        let query = IntrospectionQuery::new()
            .full_type_info(true)
            .build();

        assert!(query.contains("IntrospectionQuery"));
        assert!(query.contains("__schema"));
    }

    #[test]
    fn test_format_type_ref_simple() {
        let type_ref = json!({
            "kind": "SCALAR",
            "name": "String"
        });
        assert_eq!(format_type_ref(&type_ref), "String");
    }

    #[test]
    fn test_format_type_ref_non_null() {
        let type_ref = json!({
            "kind": "NON_NULL",
            "ofType": {
                "kind": "SCALAR",
                "name": "ID"
            }
        });
        assert_eq!(format_type_ref(&type_ref), "ID!");
    }

    #[test]
    fn test_format_type_ref_list() {
        let type_ref = json!({
            "kind": "LIST",
            "ofType": {
                "kind": "NON_NULL",
                "ofType": {
                    "kind": "OBJECT",
                    "name": "User"
                }
            }
        });
        assert_eq!(format_type_ref(&type_ref), "[User!]");
    }

    #[test]
    fn test_schema_info_from_response() {
        let response = json!({
            "data": {
                "__schema": {
                    "queryType": {"name": "Query"},
                    "mutationType": {"name": "Mutation"},
                    "subscriptionType": null,
                    "types": [
                        {
                            "kind": "OBJECT",
                            "name": "Query",
                            "fields": [
                                {
                                    "name": "users",
                                    "type": {"kind": "LIST", "ofType": {"kind": "OBJECT", "name": "User"}}
                                }
                            ]
                        }
                    ]
                }
            }
        });

        let schema = SchemaInfo::from_response(&response).expect("Should parse valid schema");
        assert_eq!(schema.query_type, Some("Query".to_string()));
        assert_eq!(schema.mutation_type, Some("Mutation".to_string()));
        assert!(schema.subscription_type.is_none());
    }

    #[test]
    fn test_schema_info_error_on_graphql_errors() {
        let response = json!({
            "errors": [
                {"message": "Introspection is disabled"}
            ],
            "data": null
        });

        let result = SchemaInfo::from_response(&response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Introspection is disabled"));
    }

    #[test]
    fn test_schema_info_error_on_missing_data() {
        let response = json!({
            "something_else": true
        });

        let result = SchemaInfo::from_response(&response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing 'data' field"));
    }
}
