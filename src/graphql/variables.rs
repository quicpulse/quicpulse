//! GraphQL variables handling
//!
//! This module provides utilities for parsing and manipulating
//! GraphQL query variables.

use serde_json::{json, Value as JsonValue, Map};
use crate::errors::QuicpulseError;

/// GraphQL variables container
#[derive(Debug, Clone, Default)]
pub struct Variables {
    inner: Map<String, JsonValue>,
}

impl Variables {
    /// Create a new empty variables container
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a JSON object
    pub fn from_json(value: JsonValue) -> Result<Self, QuicpulseError> {
        match value {
            JsonValue::Object(map) => Ok(Self { inner: map }),
            JsonValue::Null => Ok(Self::new()),
            _ => Err(QuicpulseError::Argument(
                "Variables must be a JSON object".to_string(),
            )),
        }
    }

    /// Parse variables from a string (JSON format)
    pub fn from_str(s: &str) -> Result<Self, QuicpulseError> {
        if s.trim().is_empty() {
            return Ok(Self::new());
        }

        let value: JsonValue = serde_json::from_str(s)
            .map_err(|e| QuicpulseError::Argument(format!("Invalid JSON: {}", e)))?;

        Self::from_json(value)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get number of variables
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get a variable by name
    pub fn get(&self, name: &str) -> Option<&JsonValue> {
        self.inner.get(name)
    }

    /// Set a variable
    pub fn set(&mut self, name: impl Into<String>, value: JsonValue) {
        self.inner.insert(name.into(), value);
    }

    /// Remove a variable
    pub fn remove(&mut self, name: &str) -> Option<JsonValue> {
        self.inner.remove(name)
    }

    /// Merge with another Variables instance (other takes precedence)
    pub fn merge(&mut self, other: Variables) {
        for (k, v) in other.inner {
            self.inner.insert(k, v);
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(self.inner.clone())
    }

    /// Convert to JSON string
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_else(|_| "{}".to_string())
    }

    /// Iterate over variables
    pub fn iter(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.inner.iter()
    }
}

impl IntoIterator for Variables {
    type Item = (String, JsonValue);
    type IntoIter = serde_json::map::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl From<Map<String, JsonValue>> for Variables {
    fn from(map: Map<String, JsonValue>) -> Self {
        Self { inner: map }
    }
}

impl From<Variables> for JsonValue {
    fn from(vars: Variables) -> Self {
        JsonValue::Object(vars.inner)
    }
}

/// Parse a variable definition string like "$id: Int!"
#[derive(Debug, Clone)]
pub struct VariableDefinition {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub default_value: Option<JsonValue>,
}

impl VariableDefinition {
    /// Parse from a string like "$id: Int!" or "$name: String = \"default\""
    pub fn parse(s: &str) -> Result<Self, QuicpulseError> {
        let s = s.trim();

        // Split on '=' for default value
        let (def_part, default) = if let Some(idx) = s.find('=') {
            let (left, right) = s.split_at(idx);
            let default_str = right[1..].trim();
            let default_val: JsonValue = serde_json::from_str(default_str)
                .map_err(|e| QuicpulseError::Argument(format!("Invalid default value: {}", e)))?;
            (left.trim(), Some(default_val))
        } else {
            (s, None)
        };

        // Split name and type
        let parts: Vec<&str> = def_part.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(QuicpulseError::Argument(
                format!("Invalid variable definition: {}", s),
            ));
        }

        let name = parts[0].trim().trim_start_matches('$').to_string();
        let type_str = parts[1].trim();
        let required = type_str.ends_with('!');
        let type_name = type_str.trim_end_matches('!').to_string();

        Ok(Self {
            name,
            type_name,
            required,
            default_value: default,
        })
    }
}

/// Convert a CLI argument value to the appropriate JSON type
pub fn parse_variable_value(value: &str, type_hint: Option<&str>) -> JsonValue {
    // If there's a type hint, use it
    if let Some(hint) = type_hint {
        let hint = hint.trim_end_matches('!').to_lowercase();
        match hint.as_str() {
            "int" | "integer" => {
                if let Ok(n) = value.parse::<i64>() {
                    return json!(n);
                }
            }
            "float" | "double" | "number" => {
                if let Ok(n) = value.parse::<f64>() {
                    return json!(n);
                }
            }
            "bool" | "boolean" => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "yes" => return json!(true),
                    "false" | "0" | "no" => return json!(false),
                    _ => {}
                }
            }
            "string" => return json!(value),
            _ => {}
        }
    }

    // Auto-detect type
    // Try integer
    if let Ok(n) = value.parse::<i64>() {
        return json!(n);
    }

    // Try float
    if let Ok(n) = value.parse::<f64>() {
        return json!(n);
    }

    // Try boolean
    match value.to_lowercase().as_str() {
        "true" => return json!(true),
        "false" => return json!(false),
        "null" => return JsonValue::Null,
        _ => {}
    }

    // Try JSON
    if value.starts_with('{') || value.starts_with('[') {
        if let Ok(v) = serde_json::from_str(value) {
            return v;
        }
    }

    // Default to string
    json!(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variables_new() {
        let vars = Variables::new();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_variables_from_json() {
        let json = json!({"id": 123, "name": "test"});
        let vars = Variables::from_json(json).unwrap();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("id").unwrap(), &json!(123));
    }

    #[test]
    fn test_variables_from_str() {
        let vars = Variables::from_str(r#"{"id": 123}"#).unwrap();
        assert_eq!(vars.get("id").unwrap(), &json!(123));
    }

    #[test]
    fn test_variables_merge() {
        let mut vars1 = Variables::from_json(json!({"a": 1, "b": 2})).unwrap();
        let vars2 = Variables::from_json(json!({"b": 3, "c": 4})).unwrap();
        vars1.merge(vars2);

        assert_eq!(vars1.get("a").unwrap(), &json!(1));
        assert_eq!(vars1.get("b").unwrap(), &json!(3)); // overwritten
        assert_eq!(vars1.get("c").unwrap(), &json!(4));
    }

    #[test]
    fn test_variable_definition_parse() {
        let def = VariableDefinition::parse("$id: Int!").unwrap();
        assert_eq!(def.name, "id");
        assert_eq!(def.type_name, "Int");
        assert!(def.required);
        assert!(def.default_value.is_none());
    }

    #[test]
    fn test_variable_definition_with_default() {
        let def = VariableDefinition::parse("$limit: Int = 10").unwrap();
        assert_eq!(def.name, "limit");
        assert_eq!(def.type_name, "Int");
        assert!(!def.required);
        assert_eq!(def.default_value, Some(json!(10)));
    }

    #[test]
    fn test_parse_variable_value() {
        assert_eq!(parse_variable_value("123", None), json!(123));
        assert_eq!(parse_variable_value("12.5", None), json!(12.5));
        assert_eq!(parse_variable_value("true", None), json!(true));
        assert_eq!(parse_variable_value("hello", None), json!("hello"));
        assert_eq!(parse_variable_value("[1,2,3]", None), json!([1,2,3]));
    }

    #[test]
    fn test_parse_variable_value_with_hint() {
        assert_eq!(parse_variable_value("42", Some("String")), json!("42"));
        assert_eq!(parse_variable_value("1", Some("Boolean")), json!(true));
    }
}
