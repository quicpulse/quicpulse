//! Schema module for JavaScript
//!
//! Provides JSON Schema validation.

use rquickjs::{Ctx, Object, Function};
use jsonschema::Validator;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let schema = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create schema object: {}", e)))?;

    schema.set("validate", Function::new(ctx.clone(), schema_validate)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    schema.set("is_valid", Function::new(ctx.clone(), schema_is_valid)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    schema.set("errors", Function::new(ctx.clone(), schema_errors)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("schema", schema)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set schema global: {}", e)))?;

    Ok(())
}

/// Validate JSON data against a JSON Schema
/// Returns a JSON object with `valid` boolean and optional `errors` array
fn schema_validate(schema_json: String, data_json: String) -> String {
    let schema: serde_json::Value = match serde_json::from_str(&schema_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({
                "valid": false,
                "errors": [format!("Invalid schema JSON: {}", e)]
            }).to_string();
        }
    };

    let data: serde_json::Value = match serde_json::from_str(&data_json) {
        Ok(d) => d,
        Err(e) => {
            return serde_json::json!({
                "valid": false,
                "errors": [format!("Invalid data JSON: {}", e)]
            }).to_string();
        }
    };

    let validator = match Validator::new(&schema) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "valid": false,
                "errors": [format!("Invalid schema: {}", e)]
            }).to_string();
        }
    };

    let errors: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| e.to_string())
        .collect();

    if errors.is_empty() {
        serde_json::json!({
            "valid": true
        }).to_string()
    } else {
        serde_json::json!({
            "valid": false,
            "errors": errors
        }).to_string()
    }
}

/// Check if JSON data is valid against a schema (returns boolean)
fn schema_is_valid(schema_json: String, data_json: String) -> bool {
    let schema: serde_json::Value = match serde_json::from_str(&schema_json) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let data: serde_json::Value = match serde_json::from_str(&data_json) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let validator = match Validator::new(&schema) {
        Ok(v) => v,
        Err(_) => return false,
    };

    validator.is_valid(&data)
}

/// Get validation errors as JSON array
fn schema_errors(schema_json: String, data_json: String) -> String {
    let schema: serde_json::Value = match serde_json::from_str(&schema_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!([format!("Invalid schema JSON: {}", e)]).to_string();
        }
    };

    let data: serde_json::Value = match serde_json::from_str(&data_json) {
        Ok(d) => d,
        Err(e) => {
            return serde_json::json!([format!("Invalid data JSON: {}", e)]).to_string();
        }
    };

    let validator = match Validator::new(&schema) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!([format!("Invalid schema: {}", e)]).to_string();
        }
    };

    let errors: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| e.to_string())
        .collect();

    serde_json::to_string(&errors).unwrap_or_else(|_| "[]".to_string())
}
