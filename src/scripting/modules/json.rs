//! JSON module for Rune scripts
//!
//! Provides JSON parsing, manipulation, and querying capabilities.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use serde_json::Value as JsonValue;

/// Create the json module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("json")?;

    // Parsing and serialization
    module.function("parse", parse).build()?;
    module.function("stringify", stringify).build()?;
    module.function("stringify_pretty", stringify_pretty).build()?;

    // JSONPath queries
    module.function("query", query).build()?;
    module.function("query_first", query_first).build()?;

    // Object/array operations
    module.function("get", get_value).build()?;
    module.function("keys", get_keys).build()?;
    module.function("values", get_values).build()?;
    module.function("len", get_length).build()?;
    module.function("has", has_key).build()?;

    // Type checking
    module.function("is_object", is_object).build()?;
    module.function("is_array", is_array).build()?;
    module.function("is_string", is_string).build()?;
    module.function("is_number", is_number).build()?;
    module.function("is_bool", is_bool).build()?;
    module.function("is_null", is_null).build()?;
    module.function("type_of", type_of).build()?;

    // Manipulation
    module.function("merge", merge).build()?;
    module.function("set", set_value).build()?;
    module.function("remove", remove_key).build()?;

    // Comparison
    module.function("equals", equals).build()?;
    module.function("diff", diff).build()?;

    Ok(module)
}

/// Parse a JSON string into a JSON value (returned as string)
fn parse(input: &str) -> RuneString {
    match serde_json::from_str::<JsonValue>(input) {
        Ok(value) => RuneString::try_from(value.to_string()).unwrap_or_default(),
        Err(_) => RuneString::try_from("null").unwrap_or_default(),
    }
}

/// Stringify a JSON value (pretty printed)
fn stringify(json_str: &str) -> RuneString {
    match serde_json::from_str::<JsonValue>(json_str) {
        Ok(value) => {
            match serde_json::to_string(&value) {
                Ok(s) => RuneString::try_from(s).unwrap_or_default(),
                Err(_) => RuneString::try_from(json_str.to_string()).unwrap_or_default(),
            }
        }
        Err(_) => RuneString::try_from(json_str.to_string()).unwrap_or_default(),
    }
}

/// Stringify with pretty formatting
fn stringify_pretty(json_str: &str) -> RuneString {
    match serde_json::from_str::<JsonValue>(json_str) {
        Ok(value) => {
            match serde_json::to_string_pretty(&value) {
                Ok(s) => RuneString::try_from(s).unwrap_or_default(),
                Err(_) => RuneString::try_from(json_str.to_string()).unwrap_or_default(),
            }
        }
        Err(_) => RuneString::try_from(json_str.to_string()).unwrap_or_default(),
    }
}

/// Query JSON with JSONPath expression, returns all matches as array
fn query(json_str: &str, path: &str) -> RuneString {
    use jsonpath_rust::JsonPath;

    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("[]").unwrap_or_default(),
    };

    match value.query(path) {
        Ok(results) => {
            let arr: Vec<JsonValue> = results.into_iter().cloned().collect();
            let result_arr = JsonValue::Array(arr);
            RuneString::try_from(result_arr.to_string()).unwrap_or_default()
        }
        Err(_) => {
            // Fallback: try dot-notation path
            let result = get_value_internal(&value, path);
            if result.is_null() {
                RuneString::try_from("[]").unwrap_or_default()
            } else {
                let arr = JsonValue::Array(vec![result]);
                RuneString::try_from(arr.to_string()).unwrap_or_default()
            }
        }
    }
}

/// Query JSON with JSONPath expression, returns first match
fn query_first(json_str: &str, path: &str) -> RuneString {
    use jsonpath_rust::JsonPath;

    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("null").unwrap_or_default(),
    };

    match value.query(path) {
        Ok(results) => {
            // Get first result
            if let Some(first) = results.first() {
                return RuneString::try_from(first.to_string()).unwrap_or_default();
            }
            RuneString::try_from("null").unwrap_or_default()
        }
        Err(_) => {
            // Fallback: try dot-notation path
            let result = get_value_internal(&value, path);
            RuneString::try_from(result.to_string()).unwrap_or_default()
        }
    }
}

/// Internal helper to get value by dot-notation path
fn get_value_internal(value: &JsonValue, path: &str) -> JsonValue {
    let mut current = value;
    
    let segments = parse_json_path(path);
    
    for key in segments {
        // Try as object key
        if let Some(obj) = current.as_object() {
            if let Some(v) = obj.get(&key) {
                current = v;
                continue;
            }
        }
        // Try as array index
        if let Some(arr) = current.as_array() {
            if let Ok(idx) = key.parse::<usize>() {
                if let Some(v) = arr.get(idx) {
                    current = v;
                    continue;
                }
            }
        }
        return JsonValue::Null;
    }
    current.clone()
}

/// Parse a JSON path into segments, supporting both dot notation and bracket notation
fn parse_json_path(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut chars = path.chars().peekable();
    let mut current_segment = String::new();
    
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !current_segment.is_empty() {
                    segments.push(current_segment);
                    current_segment = String::new();
                }
            }
            '[' => {
                if !current_segment.is_empty() {
                    segments.push(current_segment);
                    current_segment = String::new();
                }
                // Check if it's a quoted string ["key"] or just index [0]
                if chars.peek() == Some(&'"') {
                    chars.next(); // consume opening quote
                    let mut key = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '"' {
                            // Look for closing bracket
                            if chars.peek() == Some(&']') {
                                chars.next();
                            }
                            break;
                        }
                        if ch == '\\' {
                            // Handle escaped characters
                            if let Some(escaped) = chars.next() {
                                key.push(escaped);
                            }
                        } else {
                            key.push(ch);
                        }
                    }
                    segments.push(key);
                } else {
                    // Numeric index [0]
                    let mut idx = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == ']' {
                            break;
                        }
                        idx.push(ch);
                    }
                    segments.push(idx);
                }
            }
            ']' => {} // Already handled
            _ => {
                current_segment.push(c);
            }
        }
    }
    
    if !current_segment.is_empty() {
        segments.push(current_segment);
    }
    
    segments
}

/// Get a value by dot-notation path (e.g., "user.name")
fn get_value(json_str: &str, path: &str) -> RuneString {
    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("null").unwrap_or_default(),
    };

    let mut current = &value;
    for key in path.split('.') {
        // Try as object key
        if let Some(obj) = current.as_object() {
            if let Some(v) = obj.get(key) {
                current = v;
                continue;
            }
        }
        // Try as array index
        if let Some(arr) = current.as_array() {
            if let Ok(idx) = key.parse::<usize>() {
                if let Some(v) = arr.get(idx) {
                    current = v;
                    continue;
                }
            }
        }
        return RuneString::try_from("null").unwrap_or_default();
    }

    RuneString::try_from(current.to_string()).unwrap_or_default()
}

/// Get all keys from a JSON object
fn get_keys(json_str: &str) -> RuneString {
    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("[]").unwrap_or_default(),
    };

    if let Some(obj) = value.as_object() {
        let keys: Vec<JsonValue> = obj.keys()
            .map(|k| JsonValue::String(k.clone()))
            .collect();
        return RuneString::try_from(serde_json::to_string(&keys).unwrap_or("[]".to_string())).unwrap_or_default();
    }

    RuneString::try_from("[]").unwrap_or_default()
}

/// Get all values from a JSON object or array
fn get_values(json_str: &str) -> RuneString {
    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("[]").unwrap_or_default(),
    };

    if let Some(obj) = value.as_object() {
        let values: Vec<&JsonValue> = obj.values().collect();
        return RuneString::try_from(serde_json::to_string(&values).unwrap_or("[]".to_string())).unwrap_or_default();
    }

    if let Some(arr) = value.as_array() {
        return RuneString::try_from(serde_json::to_string(arr).unwrap_or("[]".to_string())).unwrap_or_default();
    }

    RuneString::try_from("[]").unwrap_or_default()
}

/// Get length of array or object
fn get_length(json_str: &str) -> i64 {
    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return 0,
    };

    match value {
        JsonValue::Array(arr) => arr.len() as i64,
        JsonValue::Object(obj) => obj.len() as i64,
        JsonValue::String(s) => s.len() as i64,
        _ => 0,
    }
}

/// Check if object has a key
fn has_key(json_str: &str, key: &str) -> bool {
    let value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(obj) = value.as_object() {
        return obj.contains_key(key);
    }
    false
}

/// Check if value is an object
fn is_object(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_object())
        .unwrap_or(false)
}

/// Check if value is an array
fn is_array(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_array())
        .unwrap_or(false)
}

/// Check if value is a string
fn is_string(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_string())
        .unwrap_or(false)
}

/// Check if value is a number
fn is_number(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_number())
        .unwrap_or(false)
}

/// Check if value is a boolean
fn is_bool(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_boolean())
        .unwrap_or(false)
}

/// Check if value is null
fn is_null(json_str: &str) -> bool {
    serde_json::from_str::<JsonValue>(json_str)
        .map(|v| v.is_null())
        .unwrap_or(false)
}

/// Get the type of a JSON value
fn type_of(json_str: &str) -> RuneString {
    let type_name = match serde_json::from_str::<JsonValue>(json_str) {
        Ok(JsonValue::Null) => "null",
        Ok(JsonValue::Bool(_)) => "boolean",
        Ok(JsonValue::Number(_)) => "number",
        Ok(JsonValue::String(_)) => "string",
        Ok(JsonValue::Array(_)) => "array",
        Ok(JsonValue::Object(_)) => "object",
        Err(_) => "invalid",
    };
    RuneString::try_from(type_name.to_string()).unwrap_or_default()
}

fn merge(base_str: &str, overlay_str: &str) -> RuneString {
    let mut base: JsonValue = match serde_json::from_str(base_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from(overlay_str.to_string()).unwrap_or_default(),
    };

    let overlay: JsonValue = match serde_json::from_str(overlay_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from(base_str.to_string()).unwrap_or_default(),
    };

    deep_merge(&mut base, overlay);

    RuneString::try_from(base.to_string()).unwrap_or_default()
}

fn deep_merge(base: &mut JsonValue, overlay: JsonValue) {
    match (base.as_object_mut(), overlay) {
        (Some(base_obj), JsonValue::Object(overlay_obj)) => {
            for (key, overlay_val) in overlay_obj {
                if let Some(base_val) = base_obj.get_mut(&key) {
                    if base_val.is_object() && overlay_val.is_object() {
                        deep_merge(base_val, overlay_val);
                    } else {
                        *base_val = overlay_val;
                    }
                } else {
                    base_obj.insert(key, overlay_val);
                }
            }
        }
        (None, overlay_val) => {
            *base = overlay_val;
        }
        _ => {}
    }
}

/// Set a value at a dot-notation path
fn set_value(json_str: &str, path: &str, value_str: &str) -> RuneString {
    let mut root: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => JsonValue::Object(serde_json::Map::new()),
    };

    let new_value: JsonValue = serde_json::from_str(value_str)
        .unwrap_or(JsonValue::String(value_str.to_string()));

    let keys: Vec<&str> = path.split('.').collect();
    set_nested(&mut root, &keys, new_value);

    RuneString::try_from(root.to_string()).unwrap_or_default()
}

fn set_nested(current: &mut JsonValue, keys: &[&str], value: JsonValue) {
    if keys.is_empty() {
        return;
    }

    if keys.len() == 1 {
        if let Some(obj) = current.as_object_mut() {
            obj.insert(keys[0].to_string(), value);
        }
        return;
    }

    if let Some(obj) = current.as_object_mut() {
        let next = obj.entry(keys[0].to_string())
            .or_insert(JsonValue::Object(serde_json::Map::new()));
        set_nested(next, &keys[1..], value);
    }
}

/// Remove a key from an object
fn remove_key(json_str: &str, key: &str) -> RuneString {
    let mut value: JsonValue = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from(json_str.to_string()).unwrap_or_default(),
    };

    if let Some(obj) = value.as_object_mut() {
        obj.remove(key);
    }

    RuneString::try_from(value.to_string()).unwrap_or_default()
}

/// Check if two JSON values are equal
fn equals(a: &str, b: &str) -> bool {
    let va: JsonValue = match serde_json::from_str(a) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let vb: JsonValue = match serde_json::from_str(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    va == vb
}

/// Get differences between two JSON objects
fn diff(a: &str, b: &str) -> RuneString {
    use serde_json::json;

    let va: JsonValue = match serde_json::from_str(a) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("{}").unwrap_or_default(),
    };
    let vb: JsonValue = match serde_json::from_str(b) {
        Ok(v) => v,
        Err(_) => return RuneString::try_from("{}").unwrap_or_default(),
    };

    let (obj_a, obj_b) = match (va.as_object(), vb.as_object()) {
        (Some(a), Some(b)) => (a, b),
        _ => return RuneString::try_from("{}").unwrap_or_default(),
    };

    let mut added = serde_json::Map::new();
    let mut removed = serde_json::Map::new();
    let mut changed = serde_json::Map::new();

    // Find added and changed
    for (key, val_b) in obj_b {
        if let Some(val_a) = obj_a.get(key) {
            if val_a != val_b {
                changed.insert(key.clone(), json!({"from": val_a, "to": val_b}));
            }
        } else {
            added.insert(key.clone(), val_b.clone());
        }
    }

    // Find removed
    for (key, val_a) in obj_a {
        if !obj_b.contains_key(key) {
            removed.insert(key.clone(), val_a.clone());
        }
    }

    let result = json!({
        "added": added,
        "removed": removed,
        "changed": changed,
    });

    RuneString::try_from(result.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let result = parse(r#"{"name": "test"}"#);
        assert!(result.contains("test"));
    }

    #[test]
    fn test_get_value() {
        let json = r#"{"user": {"name": "John", "age": 30}}"#;
        let name = get_value(json, "user.name");
        assert!(name.contains("John"));
    }

    #[test]
    fn test_type_of() {
        assert_eq!(type_of("null").as_str(), "null");
        assert_eq!(type_of("true").as_str(), "boolean");
        assert_eq!(type_of("42").as_str(), "number");
        assert_eq!(type_of(r#""hello""#).as_str(), "string");
        assert_eq!(type_of("[]").as_str(), "array");
        assert_eq!(type_of("{}").as_str(), "object");
    }

    #[test]
    fn test_merge() {
        let a = r#"{"a": 1, "b": 2}"#;
        let b = r#"{"b": 3, "c": 4}"#;
        let merged = merge(a, b);
        assert!(merged.contains("\"a\":1"));
        assert!(merged.contains("\"b\":3"));
        assert!(merged.contains("\"c\":4"));
    }
}
