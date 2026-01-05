//! Store module for JavaScript
//!
//! Provides a global key-value store for sharing data between scripts.

use rquickjs::{Ctx, Object, Function};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use crate::errors::QuicpulseError;

// Global store shared across all script executions
static STORE: Lazy<DashMap<String, serde_json::Value>> = Lazy::new(DashMap::new);

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let store = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create store object: {}", e)))?;

    store.set("get", Function::new(ctx.clone(), store_get)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("set", Function::new(ctx.clone(), store_set)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("delete", Function::new(ctx.clone(), store_delete)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("has", Function::new(ctx.clone(), store_has)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("keys", Function::new(ctx.clone(), store_keys)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("clear", Function::new(ctx.clone(), store_clear)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("incr", Function::new(ctx.clone(), store_incr)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    store.set("decr", Function::new(ctx.clone(), store_decr)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("store", store)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set store global: {}", e)))?;

    Ok(())
}

fn store_get(key: String) -> Option<String> {
    STORE.get(&key).map(|v| {
        // For string values, return the raw string without JSON quotes
        if let Some(s) = v.as_str() {
            s.to_string()
        } else {
            v.to_string()
        }
    })
}

fn store_set(key: String, value: String) {
    let json_value: serde_json::Value = serde_json::from_str(&value)
        .unwrap_or(serde_json::Value::String(value));
    STORE.insert(key, json_value);
}

fn store_delete(key: String) -> bool {
    STORE.remove(&key).is_some()
}

fn store_has(key: String) -> bool {
    STORE.contains_key(&key)
}

fn store_keys() -> String {
    let keys: Vec<String> = STORE.iter().map(|r| r.key().clone()).collect();
    serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
}

fn store_clear() {
    STORE.clear();
}

fn store_incr(key: String) -> i64 {
    let mut value = STORE.entry(key).or_insert(serde_json::json!(0));
    let current = value.as_i64().unwrap_or(0);
    let new_value = current + 1;
    *value = serde_json::json!(new_value);
    new_value
}

fn store_decr(key: String) -> i64 {
    let mut value = STORE.entry(key).or_insert(serde_json::json!(0));
    let current = value.as_i64().unwrap_or(0);
    let new_value = current - 1;
    *value = serde_json::json!(new_value);
    new_value
}
