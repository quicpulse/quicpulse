//! Nested JSON value handling
//!
//! Provides utilities for setting values at nested paths in JSON objects,
//! supporting bracket notation like "user[name]", "items[0]", and "items[]".

use serde_json::{json, Value as JsonValue};
use winnow::prelude::*;
use winnow::combinator::{alt, repeat};
use winnow::token::take_while;
use winnow::ModalResult;
use crate::errors::QuicpulseError;

/// Maximum array index to prevent DoS via massive allocations
const MAX_ARRAY_INDEX: usize = 10000;

/// Set a value in a JSON object at a nested path
///
/// Supports:
/// - Simple keys: "name" -> {"name": value}
/// - Nested keys: "user[name]" -> {"user": {"name": value}}
/// - Array indices: "items[0]" -> {"items": [value]}
/// - Array append: "items[]" -> appends to array
pub fn set_nested_value(obj: &mut JsonValue, key: &str, value: JsonValue) -> Result<(), QuicpulseError> {
    // Simple case: no brackets
    if !key.contains('[') {
        if let Some(map) = obj.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        return Ok(());
    }

    let tokens = parse_path(key)?;
    if tokens.is_empty() {
        return Ok(());
    }

    // Handle root array syntax: [0]=value, []=value
    if let Some(first_token) = tokens.first() {
        match first_token {
            PathToken::Index(idx) => {
                return handle_root_array_index(obj, &tokens, *idx, value);
            }
            PathToken::Append => {
                return handle_root_array_append(obj, &tokens, value);
            }
            _ => {}
        }
    }

    // Handle simple array append like "items[]"
    if tokens.len() == 2 {
        if let (PathToken::Key(first_key), PathToken::Append) = (&tokens[0], &tokens[1]) {
            return handle_simple_array_append(obj, first_key, value);
        }
    }

    // Check if path contains an Append token for merge mode
    let has_append = tokens.iter().any(|t| matches!(t, PathToken::Append));

    // Build nested structure and merge
    let nested = build_nested_value(&tokens, value)?;

    if let Some(PathToken::Key(first_key)) = tokens.first() {
        if let Some(map) = obj.as_object_mut() {
            if let Some(existing) = map.get_mut(first_key) {
                merge_deep(existing, nested.get(first_key).cloned().unwrap_or(json!(null)), has_append);
            } else {
                map.insert(first_key.clone(), nested.get(first_key).cloned().unwrap_or(json!(null)));
            }
        }
    }

    Ok(())
}

fn handle_root_array_index(obj: &mut JsonValue, tokens: &[PathToken], idx: usize, value: JsonValue) -> Result<(), QuicpulseError> {
    if obj.is_object() && obj.as_object().map(|m| m.is_empty()).unwrap_or(true) {
        *obj = json!([]);
    }
    if let Some(arr) = obj.as_array_mut() {
        while arr.len() <= idx {
            arr.push(JsonValue::Null);
        }
        if tokens.len() > 1 {
            arr[idx] = build_nested_value(&tokens[1..], value)?;
        } else {
            arr[idx] = value;
        }
    }
    Ok(())
}

fn handle_root_array_append(obj: &mut JsonValue, tokens: &[PathToken], value: JsonValue) -> Result<(), QuicpulseError> {
    if obj.is_object() && obj.as_object().map(|m| m.is_empty()).unwrap_or(true) {
        *obj = json!([]);
    }
    if let Some(arr) = obj.as_array_mut() {
        if tokens.len() > 1 {
            arr.push(build_nested_value(&tokens[1..], value)?);
        } else {
            arr.push(value);
        }
    }
    Ok(())
}

fn handle_simple_array_append(obj: &mut JsonValue, key: &str, value: JsonValue) -> Result<(), QuicpulseError> {
    if let Some(map) = obj.as_object_mut() {
        if let Some(existing) = map.get_mut(key) {
            if let Some(arr) = existing.as_array_mut() {
                arr.push(value);
                return Ok(());
            }
        } else {
            map.insert(key.to_string(), json!([value]));
            return Ok(());
        }
    }
    Ok(())
}

/// Path token for nested key parsing
#[derive(Debug, Clone, PartialEq)]
enum PathToken {
    Key(String),
    Index(usize),
    Append,
}

/// Parse a bracketed token: [0], [], or [key]
fn parse_bracketed_token(input: &mut &str) -> ModalResult<PathToken> {
    // Consume opening bracket
    '['.parse_next(input)?;

    // Parse content between brackets
    let content: &str = take_while(0.., |c: char| c != ']').parse_next(input)?;

    // Consume closing bracket
    ']'.parse_next(input)?;

    // Determine token type based on content
    if content.is_empty() {
        Ok(PathToken::Append)
    } else if let Ok(idx) = content.parse::<usize>() {
        Ok(PathToken::Index(idx))
    } else {
        Ok(PathToken::Key(content.to_string()))
    }
}

/// Parse a bare key (not in brackets)
fn parse_bare_key(input: &mut &str) -> ModalResult<PathToken> {
    let key: &str = take_while(1.., |c: char| c != '[' && c != ']')
        .parse_next(input)?;
    Ok(PathToken::Key(key.to_string()))
}

/// Parse a single path token (either bracketed or bare key)
fn parse_path_token(input: &mut &str) -> ModalResult<PathToken> {
    alt((parse_bracketed_token, parse_bare_key)).parse_next(input)
}

/// Parse a full path into tokens using winnow parser combinators
fn parse_path_winnow(input: &mut &str) -> ModalResult<Vec<PathToken>> {
    repeat(1.., parse_path_token).parse_next(input)
}

/// Parse a path like "user[name]" or "items[0]" or "items[]"
fn parse_path(key: &str) -> Result<Vec<PathToken>, QuicpulseError> {
    let mut input = key;
    let tokens = parse_path_winnow(&mut input)
        .map_err(|e| QuicpulseError::Argument(format!("Invalid path syntax: {}", e)))?;

    // Validate array indices don't exceed maximum
    for token in &tokens {
        if let PathToken::Index(idx) = token {
            if *idx > MAX_ARRAY_INDEX {
                return Err(QuicpulseError::Argument(format!(
                    "Array index {} exceeds maximum allowed ({})",
                    idx, MAX_ARRAY_INDEX
                )));
            }
        }
    }

    Ok(tokens)
}

/// Build a nested JSON value from tokens (inside-out)
fn build_nested_value(tokens: &[PathToken], value: JsonValue) -> Result<JsonValue, QuicpulseError> {
    if tokens.is_empty() {
        return Ok(value);
    }

    let mut result = value;

    for token in tokens.iter().rev() {
        match token {
            PathToken::Key(k) => {
                let mut obj = json!({});
                obj.as_object_mut().unwrap().insert(k.clone(), result);
                result = obj;
            }
            PathToken::Index(idx) => {
                let mut arr = vec![JsonValue::Null; *idx + 1];
                arr[*idx] = result;
                result = JsonValue::Array(arr);
            }
            PathToken::Append => {
                result = json!([result]);
            }
        }
    }

    Ok(result)
}

/// Deeply merge two JSON values
fn merge_deep(base: &mut JsonValue, overlay: JsonValue, append_mode: bool) {
    match (base, overlay) {
        (JsonValue::Object(base_map), JsonValue::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    merge_deep(base_value, value, append_mode);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (JsonValue::Array(base_arr), JsonValue::Array(overlay_arr)) => {
            if append_mode {
                for value in overlay_arr {
                    if !value.is_null() {
                        base_arr.push(value);
                    }
                }
            } else {
                for (i, value) in overlay_arr.into_iter().enumerate() {
                    if !value.is_null() {
                        if i < base_arr.len() {
                            if base_arr[i].is_null() {
                                base_arr[i] = value;
                            } else {
                                merge_deep(&mut base_arr[i], value, append_mode);
                            }
                        } else {
                            while base_arr.len() < i {
                                base_arr.push(JsonValue::Null);
                            }
                            base_arr.push(value);
                        }
                    }
                }
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key() {
        let mut obj = json!({});
        set_nested_value(&mut obj, "name", json!("John")).unwrap();
        assert_eq!(obj, json!({"name": "John"}));
    }

    #[test]
    fn test_nested_key() {
        let mut obj = json!({});
        set_nested_value(&mut obj, "user[name]", json!("John")).unwrap();
        assert_eq!(obj, json!({"user": {"name": "John"}}));
    }

    #[test]
    fn test_array_index() {
        let mut obj = json!({});
        set_nested_value(&mut obj, "items[0]", json!("first")).unwrap();
        assert_eq!(obj, json!({"items": ["first"]}));
    }

    #[test]
    fn test_array_append() {
        let mut obj = json!({"items": []});
        set_nested_value(&mut obj, "items[]", json!("a")).unwrap();
        set_nested_value(&mut obj, "items[]", json!("b")).unwrap();
        assert_eq!(obj, json!({"items": ["a", "b"]}));
    }

    #[test]
    fn test_deep_nesting() {
        let mut obj = json!({});
        set_nested_value(&mut obj, "a[b][c]", json!("deep")).unwrap();
        assert_eq!(obj, json!({"a": {"b": {"c": "deep"}}}));
    }

    #[test]
    fn test_root_array() {
        let mut obj = json!({});
        set_nested_value(&mut obj, "[0]", json!("first")).unwrap();
        assert_eq!(obj, json!(["first"]));
    }

    #[test]
    fn test_max_index_protection() {
        let mut obj = json!({});
        let result = set_nested_value(&mut obj, "items[999999999]", json!("bad"));
        assert!(result.is_err());
    }
}
