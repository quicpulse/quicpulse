//! JSON module for JavaScript
//!
//! Provides JSON manipulation functions.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let json = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create json object: {}", e)))?;

    // Note: JavaScript already has JSON.parse and JSON.stringify built-in
    // We add additional utilities here
    json.set("query", Function::new(ctx.clone(), json_query)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    json.set("is_valid", Function::new(ctx.clone(), json_is_valid)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    json.set("pretty", Function::new(ctx.clone(), json_pretty)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    json.set("compact", Function::new(ctx.clone(), json_compact)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    json.set("type_of", Function::new(ctx.clone(), json_type_of)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("json", json)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set json global: {}", e)))?;

    Ok(())
}

fn json_query(json_str: String, path: String) -> Option<String> {
    use jsonpath_rust::JsonPath;

    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    match json.query(&path) {
        Ok(results) => {
            if results.is_empty() {
                None
            } else if results.len() == 1 {
                Some(results[0].to_string())
            } else {
                let arr: Vec<serde_json::Value> = results.into_iter().cloned().collect();
                Some(serde_json::to_string(&arr).unwrap_or_default())
            }
        }
        Err(_) => None,
    }
}

fn json_is_valid(json_str: String) -> bool {
    serde_json::from_str::<serde_json::Value>(&json_str).is_ok()
}

fn json_pretty(json_str: String) -> String {
    serde_json::from_str::<serde_json::Value>(&json_str)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or(json_str)
}

fn json_compact(json_str: String) -> String {
    serde_json::from_str::<serde_json::Value>(&json_str)
        .and_then(|v| serde_json::to_string(&v))
        .unwrap_or(json_str)
}

fn json_type_of(json_str: String) -> String {
    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(serde_json::Value::Null) => "null".to_string(),
        Ok(serde_json::Value::Bool(_)) => "boolean".to_string(),
        Ok(serde_json::Value::Number(_)) => "number".to_string(),
        Ok(serde_json::Value::String(_)) => "string".to_string(),
        Ok(serde_json::Value::Array(_)) => "array".to_string(),
        Ok(serde_json::Value::Object(_)) => "object".to_string(),
        Err(_) => "invalid".to_string(),
    }
}
