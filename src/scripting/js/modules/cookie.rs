//! Cookie module for JavaScript
//!
//! Provides HTTP cookie parsing and building utilities.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let cookie = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create cookie object: {}", e)))?;

    cookie.set("parse", Function::new(ctx.clone(), cookie_parse)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    cookie.set("parse_set_cookie", Function::new(ctx.clone(), cookie_parse_set_cookie)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    cookie.set("build", Function::new(ctx.clone(), cookie_build)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    cookie.set("get", Function::new(ctx.clone(), cookie_get)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("cookie", cookie)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set cookie global: {}", e)))?;

    Ok(())
}

/// Parse a Cookie header value into a JSON object
fn cookie_parse(header: String) -> String {
    let mut cookies = serde_json::Map::new();

    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some((name, value)) = pair.split_once('=') {
            cookies.insert(
                name.trim().to_string(),
                serde_json::Value::String(value.trim().to_string()),
            );
        }
    }

    serde_json::Value::Object(cookies).to_string()
}

/// Parse a Set-Cookie header value into a detailed JSON object
fn cookie_parse_set_cookie(header: String) -> String {
    let mut result = serde_json::Map::new();
    let parts: Vec<&str> = header.split(';').collect();

    // First part is name=value
    if let Some(first) = parts.first() {
        if let Some((name, value)) = first.split_once('=') {
            result.insert("name".to_string(), serde_json::Value::String(name.trim().to_string()));
            result.insert("value".to_string(), serde_json::Value::String(value.trim().to_string()));
        }
    }

    // Parse attributes
    for part in parts.iter().skip(1) {
        let part = part.trim();
        let (attr_name, attr_value) = if let Some((n, v)) = part.split_once('=') {
            (n.trim().to_lowercase(), Some(v.trim().to_string()))
        } else {
            (part.to_lowercase(), None)
        };

        match attr_name.as_str() {
            "expires" => {
                if let Some(v) = attr_value {
                    result.insert("expires".to_string(), serde_json::Value::String(v));
                }
            }
            "max-age" => {
                if let Some(v) = attr_value {
                    if let Ok(age) = v.parse::<i64>() {
                        result.insert("max_age".to_string(), serde_json::Value::Number(age.into()));
                    }
                }
            }
            "domain" => {
                if let Some(v) = attr_value {
                    result.insert("domain".to_string(), serde_json::Value::String(v));
                }
            }
            "path" => {
                if let Some(v) = attr_value {
                    result.insert("path".to_string(), serde_json::Value::String(v));
                }
            }
            "secure" => {
                result.insert("secure".to_string(), serde_json::Value::Bool(true));
            }
            "httponly" => {
                result.insert("http_only".to_string(), serde_json::Value::Bool(true));
            }
            "samesite" => {
                if let Some(v) = attr_value {
                    result.insert("same_site".to_string(), serde_json::Value::String(v));
                }
            }
            _ => {}
        }
    }

    serde_json::Value::Object(result).to_string()
}

/// Build a cookie string from name and value
fn cookie_build(name: String, value: String) -> String {
    format!("{}={}", name, value)
}

/// Get a specific cookie value from a Cookie header
fn cookie_get(header: String, name: String) -> Option<String> {
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some((cookie_name, cookie_value)) = pair.split_once('=') {
            if cookie_name.trim() == name {
                return Some(cookie_value.trim().to_string());
            }
        }
    }
    None
}
