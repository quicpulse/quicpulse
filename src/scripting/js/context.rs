//! JavaScript context bridge
//!
//! Injects ScriptContext data (request, response, variables) into the
//! JavaScript global scope so scripts can access them.

use rquickjs::{Ctx, Object, Array, Value, IntoJs};
use std::collections::HashMap;
use serde_json::Value as JsonValue;

use crate::errors::QuicpulseError;
use crate::scripting::context::{RequestData, ResponseData};

/// Inject script context into JavaScript globals
pub fn inject_context(
    ctx: &Ctx<'_>,
    request: Option<&RequestData>,
    response: Option<&ResponseData>,
    variables: &HashMap<String, JsonValue>,
    env: &HashMap<String, String>,
) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();

    // Inject request object
    if let Some(req) = request {
        let request_obj = Object::new(ctx.clone())
            .map_err(|e| QuicpulseError::Script(format!("Failed to create request object: {}", e)))?;

        request_obj.set("method", req.method.as_str())
            .map_err(|e| QuicpulseError::Script(format!("Failed to set method: {}", e)))?;
        request_obj.set("url", req.url.as_str())
            .map_err(|e| QuicpulseError::Script(format!("Failed to set url: {}", e)))?;

        // Headers as object
        let headers_obj = hashmap_to_js_object(ctx, &req.headers)?;
        request_obj.set("headers", headers_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set headers: {}", e)))?;

        // Query params as object
        let query_obj = hashmap_to_js_object(ctx, &req.query)?;
        request_obj.set("query", query_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set query: {}", e)))?;

        // Form data as object
        let form_obj = hashmap_to_js_object(ctx, &req.form)?;
        request_obj.set("form", form_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set form: {}", e)))?;

        // Body as JSON value
        if let Some(ref body) = req.body {
            let body_val = json_to_js_value(ctx, body)?;
            request_obj.set("body", body_val)
                .map_err(|e| QuicpulseError::Script(format!("Failed to set body: {}", e)))?;
        }

        globals.set("request", request_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set request global: {}", e)))?;
    }

    // Inject response object
    if let Some(resp) = response {
        let response_obj = Object::new(ctx.clone())
            .map_err(|e| QuicpulseError::Script(format!("Failed to create response object: {}", e)))?;

        response_obj.set("status", resp.status as i32)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set status: {}", e)))?;

        // Headers as object
        let headers_obj = hashmap_to_js_object(ctx, &resp.headers)?;
        response_obj.set("headers", headers_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set headers: {}", e)))?;

        // Body as JSON value
        let body_val = json_to_js_value(ctx, &resp.body)?;
        response_obj.set("body", body_val)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set body: {}", e)))?;

        response_obj.set("elapsed_ms", resp.elapsed_ms as f64)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set elapsed_ms: {}", e)))?;

        // Convenience: status as top-level variable
        globals.set("status", resp.status as i32)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set status global: {}", e)))?;

        globals.set("response", response_obj)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set response global: {}", e)))?;
    }

    // Inject variables object
    let variables_obj = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create variables object: {}", e)))?;
    for (key, value) in variables {
        let js_val = json_to_js_value(ctx, value)?;
        variables_obj.set(key.as_str(), js_val)
            .map_err(|e| QuicpulseError::Script(format!("Failed to set variable {}: {}", key, e)))?;
    }
    globals.set("variables", variables_obj)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set variables global: {}", e)))?;

    // Inject env object (safe subset)
    let env_obj = hashmap_to_js_object(ctx, env)?;
    globals.set("env", env_obj)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set env global: {}", e)))?;

    Ok(())
}

/// Convert a HashMap<String, String> to a JavaScript object
fn hashmap_to_js_object<'js>(ctx: &Ctx<'js>, map: &HashMap<String, String>) -> Result<Object<'js>, QuicpulseError> {
    let obj = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create object: {}", e)))?;

    for (key, value) in map {
        obj.set(key.as_str(), value.as_str())
            .map_err(|e| QuicpulseError::Script(format!("Failed to set {}: {}", key, e)))?;
    }

    Ok(obj)
}

/// Convert a serde_json::Value to a QuickJS Value
fn json_to_js_value<'js>(ctx: &Ctx<'js>, json: &JsonValue) -> Result<Value<'js>, QuicpulseError> {
    match json {
        JsonValue::Null => Ok(Value::new_null(ctx.clone())),
        JsonValue::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_float(ctx.clone(), 0.0))
            }
        }
        JsonValue::String(s) => {
            s.as_str().into_js(ctx)
                .map_err(|e| QuicpulseError::Script(format!("Failed to convert string: {}", e)))
        }
        JsonValue::Array(arr) => {
            let js_arr = Array::new(ctx.clone())
                .map_err(|e| QuicpulseError::Script(format!("Failed to create array: {}", e)))?;
            for (i, item) in arr.iter().enumerate() {
                let val = json_to_js_value(ctx, item)?;
                js_arr.set(i, val)
                    .map_err(|e| QuicpulseError::Script(format!("Failed to set array item: {}", e)))?;
            }
            Ok(js_arr.into_value())
        }
        JsonValue::Object(obj) => {
            let js_obj = Object::new(ctx.clone())
                .map_err(|e| QuicpulseError::Script(format!("Failed to create object: {}", e)))?;
            for (key, value) in obj {
                let val = json_to_js_value(ctx, value)?;
                js_obj.set(key.as_str(), val)
                    .map_err(|e| QuicpulseError::Script(format!("Failed to set object key: {}", e)))?;
            }
            Ok(js_obj.into_value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Runtime, Context};

    #[test]
    fn test_inject_empty_context() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        context.with(|ctx| {
            let result = inject_context(
                &ctx,
                None,
                None,
                &HashMap::new(),
                &HashMap::new(),
            );
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_inject_with_response() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        let response = ResponseData {
            status: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: serde_json::json!({"success": true}),
            elapsed_ms: 100,
        };

        context.with(|ctx| {
            let result = inject_context(
                &ctx,
                None,
                Some(&response),
                &HashMap::new(),
                &HashMap::new(),
            );
            assert!(result.is_ok());

            // Verify status is accessible
            let status: i32 = ctx.globals().get("status").unwrap();
            assert_eq!(status, 200);
        });
    }
}
