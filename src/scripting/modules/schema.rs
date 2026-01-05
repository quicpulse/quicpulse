//! JSON Schema validation module for Rune scripts
//!
//! Provides JSON Schema validation capabilities.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use serde_json::Value as JsonValue;

/// Create the schema module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("schema")?;

    // Validation
    module.function("validate", validate_schema).build()?;
    module.function("is_valid", is_valid_against_schema).build()?;
    module.function("errors", get_validation_errors).build()?;

    // Schema helpers
    module.function("type_string", type_string).build()?;
    module.function("type_number", type_number).build()?;
    module.function("type_integer", type_integer).build()?;
    module.function("type_boolean", type_boolean).build()?;
    module.function("type_array", type_array).build()?;
    module.function("type_object", type_object).build()?;

    // Common patterns
    module.function("email", email_pattern).build()?;
    module.function("uuid", uuid_pattern).build()?;
    module.function("date", date_pattern).build()?;
    module.function("url", url_pattern).build()?;

    Ok(module)
}

/// Validate JSON against a schema and return detailed result
fn validate_schema(json_str: &str, schema_str: &str) -> RuneString {
    let instance: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            let result = serde_json::json!({
                "valid": false,
                "errors": [{"message": format!("Invalid JSON: {}", e)}]
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
    };

    let schema: JsonValue = match serde_json::from_str(schema_str) {
        Ok(v) => v,
        Err(e) => {
            let result = serde_json::json!({
                "valid": false,
                "errors": [{"message": format!("Invalid schema: {}", e)}]
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
    };

    let validator = match jsonschema::validator_for(&schema) {
        Ok(c) => c,
        Err(e) => {
            let result = serde_json::json!({
                "valid": false,
                "errors": [{"message": format!("Schema compilation error: {}", e)}]
            });
            return RuneString::try_from(result.to_string()).unwrap_or_default();
        }
    };

    // Use iter_errors to get all validation errors
    let errors: Vec<_> = validator.iter_errors(&instance).collect();

    if errors.is_empty() {
        let result = serde_json::json!({
            "valid": true,
            "errors": []
        });
        RuneString::try_from(result.to_string()).unwrap_or_default()
    } else {
        let error_list: Vec<JsonValue> = errors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "path": e.instance_path().to_string(),
                    "message": e.to_string(),
                })
            })
            .collect();

        let result = serde_json::json!({
            "valid": false,
            "errors": error_list
        });
        RuneString::try_from(result.to_string()).unwrap_or_default()
    }
}

/// Check if JSON is valid against schema (returns bool)
fn is_valid_against_schema(json_str: &str, schema_str: &str) -> bool {
    let instance: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let schema: JsonValue = match serde_json::from_str(schema_str) {
        Ok(v) => v,
        Err(_) => return false,
    };

    jsonschema::is_valid(&schema, &instance)
}

/// Get validation errors as JSON array
fn get_validation_errors(json_str: &str, schema_str: &str) -> RuneString {
    let instance: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            let errors = serde_json::json!([{"message": format!("Invalid JSON: {}", e)}]);
            return RuneString::try_from(errors.to_string()).unwrap_or_default();
        }
    };

    let schema: JsonValue = match serde_json::from_str(schema_str) {
        Ok(v) => v,
        Err(e) => {
            let errors = serde_json::json!([{"message": format!("Invalid schema: {}", e)}]);
            return RuneString::try_from(errors.to_string()).unwrap_or_default();
        }
    };

    match jsonschema::validator_for(&schema) {
        Ok(validator) => {
            let errors: Vec<_> = validator.iter_errors(&instance).collect();
            if errors.is_empty() {
                RuneString::try_from("[]").unwrap_or_default()
            } else {
                let error_list: Vec<JsonValue> = errors
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "path": e.instance_path().to_string(),
                            "message": e.to_string(),
                        })
                    })
                    .collect();
                RuneString::try_from(serde_json::to_string(&error_list).unwrap_or("[]".to_string())).unwrap_or_default()
            }
        }
        Err(e) => {
            let errors = serde_json::json!([{"message": format!("Schema error: {}", e)}]);
            RuneString::try_from(errors.to_string()).unwrap_or_default()
        }
    }
}

// Schema builder helpers - return JSON schema fragments

/// Create a string type schema
fn type_string() -> RuneString {
    RuneString::try_from(r#"{"type": "string"}"#.to_string()).unwrap_or_default()
}

/// Create a number type schema
fn type_number() -> RuneString {
    RuneString::try_from(r#"{"type": "number"}"#.to_string()).unwrap_or_default()
}

/// Create an integer type schema
fn type_integer() -> RuneString {
    RuneString::try_from(r#"{"type": "integer"}"#.to_string()).unwrap_or_default()
}

/// Create a boolean type schema
fn type_boolean() -> RuneString {
    RuneString::try_from(r#"{"type": "boolean"}"#.to_string()).unwrap_or_default()
}

/// Create an array type schema
fn type_array() -> RuneString {
    RuneString::try_from(r#"{"type": "array"}"#.to_string()).unwrap_or_default()
}

/// Create an object type schema
fn type_object() -> RuneString {
    RuneString::try_from(r#"{"type": "object"}"#.to_string()).unwrap_or_default()
}

/// Create an email format schema
fn email_pattern() -> RuneString {
    RuneString::try_from(r#"{"type": "string", "format": "email"}"#.to_string()).unwrap_or_default()
}

/// Create a UUID format schema
fn uuid_pattern() -> RuneString {
    RuneString::try_from(r#"{"type": "string", "format": "uuid"}"#.to_string()).unwrap_or_default()
}

/// Create a date format schema
fn date_pattern() -> RuneString {
    RuneString::try_from(r#"{"type": "string", "format": "date"}"#.to_string()).unwrap_or_default()
}

/// Create a URL format schema
fn url_pattern() -> RuneString {
    RuneString::try_from(r#"{"type": "string", "format": "uri"}"#.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid() {
        let json = r#"{"name": "John", "age": 30}"#;
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}}"#;

        let result = validate_schema(json, schema);
        assert!(result.contains("\"valid\":true"));
    }

    #[test]
    fn test_validate_invalid() {
        let json = r#"{"name": "John", "age": "thirty"}"#;
        let schema = r#"{"type": "object", "properties": {"age": {"type": "integer"}}}"#;

        let result = validate_schema(json, schema);
        assert!(result.contains("\"valid\":false"));
    }

    #[test]
    fn test_is_valid() {
        let json = r#"{"name": "John"}"#;
        let schema = r#"{"type": "object"}"#;
        assert!(is_valid_against_schema(json, schema));

        let invalid_json = r#"["not", "an", "object"]"#;
        assert!(!is_valid_against_schema(invalid_json, schema));
    }

    #[test]
    fn test_type_helpers() {
        assert!(type_string().contains("string"));
        assert!(type_number().contains("number"));
        assert!(type_array().contains("array"));
    }
}
