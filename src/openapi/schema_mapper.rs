//! Schema to Magic Value Mapper
//!
//! Maps OpenAPI schema types and formats to QuicPulse magic.rs template values.

use super::parser::Schema;
use serde_json::Value;
use std::collections::HashMap;

/// Maps OpenAPI schemas to magic.rs template values
pub struct SchemaMapper;

impl SchemaMapper {
    /// Generate a magic value template based on schema type and format
    pub fn schema_to_magic(schema: &Schema) -> String {
        // Handle enum values first
        if !schema.enum_values.is_empty() {
            return Self::enum_to_magic(&schema.enum_values);
        }

        // Handle example if provided
        if let Some(example) = &schema.example {
            return Self::value_to_string(example);
        }

        // Handle default if provided
        if let Some(default) = &schema.default {
            return Self::value_to_string(default);
        }

        // Map type + format to magic values
        match (schema.schema_type.as_deref(), schema.format.as_deref()) {
            // String formats
            (Some("string"), Some("uuid")) => "{uuid}".to_string(),
            (Some("string"), Some("email")) => "{email}".to_string(),
            (Some("string"), Some("date-time")) => "{now}".to_string(),
            (Some("string"), Some("date")) => "{date}".to_string(),
            (Some("string"), Some("time")) => "{time}".to_string(),
            (Some("string"), Some("uri")) => "https://example.com/{random_string:8}".to_string(),
            (Some("string"), Some("hostname")) => "example-{random_string:8}.com".to_string(),
            (Some("string"), Some("ipv4")) => "192.168.1.{random_int:1:254}".to_string(),
            (Some("string"), Some("ipv6")) => "::1".to_string(),
            (Some("string"), Some("byte")) => "{random_bytes:16}".to_string(),
            (Some("string"), Some("binary")) => "{random_bytes:32}".to_string(),
            (Some("string"), Some("password")) => "{random_string:16}".to_string(),
            (Some("string"), Some("phone")) => "+1-555-{random_int:100:999}-{random_int:1000:9999}".to_string(),

            // String with length constraints
            (Some("string"), _) => Self::string_with_constraints(schema),

            // Integer formats
            (Some("integer"), Some("int32")) => Self::int_with_constraints(schema, i32::MIN as i64, i32::MAX as i64),
            (Some("integer"), Some("int64")) => Self::int_with_constraints(schema, i64::MIN, i64::MAX),
            (Some("integer"), _) => Self::int_with_constraints(schema, 0, 1000),

            // Number formats
            (Some("number"), Some("float")) => Self::float_with_constraints(schema),
            (Some("number"), Some("double")) => Self::float_with_constraints(schema),
            (Some("number"), _) => Self::float_with_constraints(schema),

            // Boolean
            (Some("boolean"), _) => "{random_bool}".to_string(),

            // Array
            (Some("array"), _) => Self::array_to_magic(schema),

            // Object
            (Some("object"), _) => Self::object_to_magic(schema),

            // Unknown type - use random string
            _ => "{random_string:10}".to_string(),
        }
    }

    /// Convert enum values to a pick template
    fn enum_to_magic(values: &[Value]) -> String {
        let options: Vec<String> = values.iter()
            .map(|v| Self::value_to_string(v))
            .collect();
        format!("{{pick:{}}}", options.join(","))
    }

    /// Generate string with length constraints
    fn string_with_constraints(schema: &Schema) -> String {
        let len = match (schema.min_length, schema.max_length) {
            (Some(min), Some(max)) => (min + max) / 2,
            (Some(min), None) => min.max(10),
            (None, Some(max)) => max.min(20),
            (None, None) => 10,
        };
        format!("{{random_string:{}}}", len)
    }

    /// Generate integer with constraints
    fn int_with_constraints(schema: &Schema, type_min: i64, type_max: i64) -> String {
        let min = schema.minimum.map(|m| m as i64).unwrap_or(0);
        let max = schema.maximum.map(|m| m as i64).unwrap_or(1000);

        // Clamp to type bounds
        let min = min.max(type_min);
        let max = max.min(type_max);

        format!("{{random_int:{}:{}}}", min, max)
    }

    /// Generate float with constraints
    fn float_with_constraints(schema: &Schema) -> String {
        let min = schema.minimum.unwrap_or(0.0);
        let max = schema.maximum.unwrap_or(100.0);
        format!("{{random_float:{}:{}}}", min, max)
    }

    /// Generate array magic value
    fn array_to_magic(schema: &Schema) -> String {
        if let Some(items) = &schema.items {
            let item_value = Self::schema_to_magic(items);
            // Generate a single-element array template
            format!("[{}]", item_value)
        } else {
            "[]".to_string()
        }
    }

    /// Generate object magic value
    fn object_to_magic(schema: &Schema) -> String {
        if schema.properties.is_empty() {
            return "{}".to_string();
        }

        let mut pairs = Vec::new();
        for (name, prop_schema) in &schema.properties {
            let value = Self::schema_to_magic(prop_schema);
            // Determine if value needs quoting
            let quoted_value = if value.starts_with('{') && !value.starts_with("{\"") {
                format!("\"{}\"", value)
            } else if value.starts_with('[') || value.starts_with('{') {
                value
            } else if value.parse::<f64>().is_ok() || value == "true" || value == "false" {
                value
            } else {
                format!("\"{}\"", value)
            };
            pairs.push(format!("\"{}\": {}", name, quoted_value));
        }
        format!("{{{}}}", pairs.join(", "))
    }

    /// Convert a JSON value to string
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string()),
            Value::Object(obj) => serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    /// Generate a complete request body from a schema
    pub fn generate_request_body(schema: &Schema, schemas: &HashMap<String, Schema>) -> Value {
        Self::generate_value(schema, schemas, 0)
    }

    /// Generate a JSON value from a schema, with recursion limit
    fn generate_value(schema: &Schema, schemas: &HashMap<String, Schema>, depth: usize) -> Value {
        // Prevent infinite recursion
        if depth > 10 {
            return Value::Null;
        }

        // Handle $ref
        if let Some(ref_path) = &schema.ref_path {
            let ref_name = ref_path.rsplit('/').next().unwrap_or("");
            if let Some(ref_schema) = schemas.get(ref_name) {
                return Self::generate_value(ref_schema, schemas, depth + 1);
            }
        }

        // Handle example
        if let Some(example) = &schema.example {
            return example.clone();
        }

        // Handle enum
        if !schema.enum_values.is_empty() {
            if let Some(first) = schema.enum_values.first() {
                return first.clone();
            }
        }

        match schema.schema_type.as_deref() {
            Some("object") => {
                let mut obj = serde_json::Map::new();
                for (name, prop_schema) in &schema.properties {
                    let magic_or_value = Self::schema_to_magic(prop_schema);
                    // Check if it's a magic value or actual value
                    if magic_or_value.contains('{') && magic_or_value.contains('}') {
                        // It's a magic template, use as string
                        obj.insert(name.clone(), Value::String(magic_or_value));
                    } else {
                        // Try to parse as JSON, fallback to string
                        let value = serde_json::from_str(&magic_or_value)
                            .unwrap_or_else(|_| Value::String(magic_or_value));
                        obj.insert(name.clone(), value);
                    }
                }
                Value::Object(obj)
            }
            Some("array") => {
                if let Some(items) = &schema.items {
                    let item = Self::generate_value(items, schemas, depth + 1);
                    Value::Array(vec![item])
                } else {
                    Value::Array(vec![])
                }
            }
            Some("string") => {
                Value::String(Self::schema_to_magic(schema))
            }
            Some("integer") | Some("number") => {
                // Return the magic template as a string - will be expanded later
                Value::String(Self::schema_to_magic(schema))
            }
            Some("boolean") => {
                Value::String("{random_bool}".to_string())
            }
            _ => Value::Null,
        }
    }

    /// Map OpenAPI type to fuzz category for automated security testing
    pub fn type_to_fuzz_category(schema: &Schema) -> Vec<String> {
        let mut categories = Vec::new();

        match (schema.schema_type.as_deref(), schema.format.as_deref()) {
            // String types vulnerable to injection
            (Some("string"), None) => {
                categories.push("sql".to_string());
                categories.push("xss".to_string());
                categories.push("cmd".to_string());
            }
            (Some("string"), Some("uri" | "url")) => {
                categories.push("ssrf".to_string());
                categories.push("path".to_string());
            }
            (Some("string"), Some("email")) => {
                categories.push("format".to_string());
            }

            // Integer types - boundary testing
            (Some("integer"), _) => {
                categories.push("int".to_string());
                categories.push("boundary".to_string());
            }

            // All types get type confusion tests
            _ => {
                categories.push("type".to_string());
            }
        }

        categories
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("uuid".to_string()),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{uuid}");
    }

    #[test]
    fn test_email_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("email".to_string()),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{email}");
    }

    #[test]
    fn test_integer_with_bounds() {
        let schema = Schema {
            schema_type: Some("integer".to_string()),
            minimum: Some(1.0),
            maximum: Some(100.0),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{random_int:1:100}");
    }

    #[test]
    fn test_enum_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            enum_values: vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string()),
            ],
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{pick:active,inactive}");
    }

    #[test]
    fn test_example_takes_precedence() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            example: Some(Value::String("my-example".to_string())),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "my-example");
    }
}
