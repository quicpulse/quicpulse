//! Store module for global workflow state
//!
//! Provides a key-value store that persists across workflow steps,
//! allowing data sharing between unrelated steps.
//!
//! Uses DashMap for lock-free concurrent access, avoiding blocking
//! in async contexts (tokio runtime).

use dashmap::DashMap;
use once_cell::sync::Lazy;
use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use serde_json::Value as JsonValue;

/// Global key-value store using DashMap for non-blocking concurrent access
/// This avoids blocking tokio worker threads during bench/fuzz operations
static STORE: Lazy<DashMap<String, JsonValue>> = Lazy::new(DashMap::new);

/// Create the store module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("store")?;

    // Basic operations
    module.function("get", get_value).build()?;
    module.function("set", set_value).build()?;
    module.function("delete", delete_value).build()?;
    module.function("has", has_key).build()?;

    // String operations
    module.function("get_string", get_string).build()?;
    module.function("set_string", set_string).build()?;

    // Number operations
    module.function("get_int", get_int).build()?;
    module.function("set_int", set_int).build()?;
    module.function("get_float", get_float).build()?;
    module.function("set_float", set_float).build()?;

    // Boolean operations
    module.function("get_bool", get_bool).build()?;
    module.function("set_bool", set_bool).build()?;

    // JSON operations
    module.function("get_json", get_json).build()?;
    module.function("set_json", set_json).build()?;

    // Utility functions
    module.function("keys", get_keys).build()?;
    module.function("clear", clear_store).build()?;
    module.function("count", count_keys).build()?;

    // Increment/decrement helpers
    module.function("incr", increment).build()?;
    module.function("decr", decrement).build()?;

    // List operations (stored as JSON arrays)
    module.function("push", list_push).build()?;
    module.function("pop", list_pop).build()?;
    module.function("list_len", list_len).build()?;

    Ok(module)
}

/// Get a value as JSON string
fn get_value(key: &str) -> RuneString {
    match STORE.get(key) {
        Some(value) => RuneString::try_from(value.to_string()).unwrap_or_default(),
        None => RuneString::new(),
    }
}

/// Set a value from JSON string
fn set_value(key: &str, value: &str) {
    if let Ok(json) = serde_json::from_str(value) {
        STORE.insert(key.to_string(), json);
    } else {
        // Store as string if not valid JSON
        STORE.insert(key.to_string(), JsonValue::String(value.to_string()));
    }
}

/// Delete a key
fn delete_value(key: &str) -> bool {
    STORE.remove(key).is_some()
}

/// Check if a key exists
fn has_key(key: &str) -> bool {
    STORE.contains_key(key)
}

/// Get a value as string
fn get_string(key: &str) -> RuneString {
    match STORE.get(key) {
        Some(entry) => match entry.value() {
            JsonValue::String(s) => RuneString::try_from(s.clone()).unwrap_or_default(),
            other => RuneString::try_from(other.to_string()).unwrap_or_default(),
        },
        None => RuneString::new(),
    }
}

/// Set a string value
fn set_string(key: &str, value: &str) {
    STORE.insert(key.to_string(), JsonValue::String(value.to_string()));
}

/// Get a value as integer
fn get_int(key: &str) -> i64 {
    match STORE.get(key) {
        Some(entry) => match entry.value() {
            JsonValue::Number(n) => n.as_i64().unwrap_or(0),
            JsonValue::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        },
        None => 0,
    }
}

/// Set an integer value
fn set_int(key: &str, value: i64) {
    STORE.insert(key.to_string(), JsonValue::Number(value.into()));
}

/// Get a value as float
fn get_float(key: &str) -> f64 {
    match STORE.get(key) {
        Some(entry) => match entry.value() {
            JsonValue::Number(n) => n.as_f64().unwrap_or(0.0),
            JsonValue::String(s) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        },
        None => 0.0,
    }
}

/// Set a float value
fn set_float(key: &str, value: f64) {
    if let Some(n) = serde_json::Number::from_f64(value) {
        STORE.insert(key.to_string(), JsonValue::Number(n));
    }
}

/// Get a value as boolean
fn get_bool(key: &str) -> bool {
    match STORE.get(key) {
        Some(entry) => match entry.value() {
            JsonValue::Bool(b) => *b,
            JsonValue::String(s) => s == "true" || s == "1",
            JsonValue::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
            _ => false,
        },
        None => false,
    }
}

/// Set a boolean value
fn set_bool(key: &str, value: bool) {
    STORE.insert(key.to_string(), JsonValue::Bool(value));
}

/// Get a value as JSON string (same as get_value but explicit)
fn get_json(key: &str) -> RuneString {
    get_value(key)
}

/// Set a JSON value from string
fn set_json(key: &str, json: &str) {
    if let Ok(value) = serde_json::from_str(json) {
        STORE.insert(key.to_string(), value);
    }
}

/// Get all keys as comma-separated string
fn get_keys() -> RuneString {
    let keys: Vec<String> = STORE.iter().map(|e| e.key().clone()).collect();
    RuneString::try_from(keys.join(",")).unwrap_or_default()
}

/// Clear all values from the store
fn clear_store() {
    STORE.clear();
}

/// Count the number of keys in the store
fn count_keys() -> i64 {
    STORE.len() as i64
}

/// Increment a numeric value
/// Bug #3 fix: Uses atomic entry API to prevent race condition
/// Previously used non-atomic read-modify-write which could lose updates
fn increment(key: &str) -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};

    // Use entry API for atomic update
    let key_string = key.to_string();
    let result = AtomicI64::new(0);

    STORE.entry(key_string.clone())
        .and_modify(|v| {
            if let JsonValue::Number(n) = v {
                let current = n.as_i64().unwrap_or(0);
                let new_val = current + 1;
                *v = JsonValue::Number(new_val.into());
                result.store(new_val, Ordering::SeqCst);
            } else {
                // Not a number - convert to 1
                *v = JsonValue::Number(1.into());
                result.store(1, Ordering::SeqCst);
            }
        })
        .or_insert_with(|| {
            result.store(1, Ordering::SeqCst);
            JsonValue::Number(1.into())
        });

    result.load(Ordering::SeqCst)
}

/// Decrement a numeric value
/// Bug #3 fix: Uses atomic entry API to prevent race condition
/// Previously used non-atomic read-modify-write which could lose updates
fn decrement(key: &str) -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};

    // Use entry API for atomic update
    let key_string = key.to_string();
    let result = AtomicI64::new(0);

    STORE.entry(key_string.clone())
        .and_modify(|v| {
            if let JsonValue::Number(n) = v {
                let current = n.as_i64().unwrap_or(0);
                let new_val = current - 1;
                *v = JsonValue::Number(new_val.into());
                result.store(new_val, Ordering::SeqCst);
            } else {
                // Not a number - convert to -1
                *v = JsonValue::Number((-1).into());
                result.store(-1, Ordering::SeqCst);
            }
        })
        .or_insert_with(|| {
            result.store(-1, Ordering::SeqCst);
            JsonValue::Number((-1).into())
        });

    result.load(Ordering::SeqCst)
}

/// Push a value onto a list (stored as JSON array)
fn list_push(key: &str, value: &str) {
    let value_json = serde_json::from_str(value)
        .unwrap_or(JsonValue::String(value.to_string()));

    // Use entry API for atomic update
    STORE.entry(key.to_string())
        .and_modify(|v| {
            if let JsonValue::Array(arr) = v {
                arr.push(value_json.clone());
            }
        })
        .or_insert(JsonValue::Array(vec![value_json]));
}

/// Pop a value from a list
fn list_pop(key: &str) -> RuneString {
    if let Some(mut entry) = STORE.get_mut(key) {
        if let JsonValue::Array(arr) = entry.value_mut() {
            if let Some(value) = arr.pop() {
                return RuneString::try_from(value.to_string()).unwrap_or_default();
            }
        }
    }
    RuneString::new()
}

/// Get the length of a list
fn list_len(key: &str) -> i64 {
    match STORE.get(key) {
        Some(entry) => match entry.value() {
            JsonValue::Array(arr) => arr.len() as i64,
            _ => 0,
        },
        None => 0,
    }
}

/// Clear the store (for testing/workflow reset)
pub fn reset_store() {
    clear_store();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests use unique key prefixes to avoid race conditions when running in parallel.
    // Each test operates on its own keys, so no global setup/teardown is needed.

    #[test]
    fn test_string_operations() {
        // Use unique keys for this test
        let key = "test_string_name";

        set_string(key, "John");
        assert_eq!(get_string(key).as_str(), "John");
        assert!(has_key(key));

        delete_value(key);
        assert!(!has_key(key));
    }

    #[test]
    fn test_int_operations() {
        // Use unique keys for this test
        let key = "test_int_count";

        set_int(key, 42);
        assert_eq!(get_int(key), 42);

        assert_eq!(increment(key), 43);
        assert_eq!(decrement(key), 42);

        // Cleanup
        delete_value(key);
    }

    #[test]
    fn test_list_operations() {
        // Use unique keys for this test
        let key = "test_list_items";

        // Ensure we start fresh by deleting any existing key
        delete_value(key);

        list_push(key, "\"first\"");
        list_push(key, "\"second\"");
        assert_eq!(list_len(key), 2);

        let popped = list_pop(key);
        assert!(popped.contains("second"));
        assert_eq!(list_len(key), 1);

        // Cleanup
        delete_value(key);
    }

    #[test]
    fn test_json_operations() {
        // Use unique keys for this test
        let key = "test_json_data";

        set_json(key, r#"{"key": "value", "num": 123}"#);
        let json = get_json(key);
        assert!(json.contains("value"));
        assert!(json.contains("123"));

        // Cleanup
        delete_value(key);
    }
}
