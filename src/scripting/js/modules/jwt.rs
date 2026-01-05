//! JWT module for JavaScript
//!
//! Provides JWT decoding and inspection utilities.

use rquickjs::{Ctx, Object, Function};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let jwt = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create jwt object: {}", e)))?;

    jwt.set("decode", Function::new(ctx.clone(), jwt_decode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("header", Function::new(ctx.clone(), jwt_header)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("payload", Function::new(ctx.clone(), jwt_payload)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("is_expired", Function::new(ctx.clone(), jwt_is_expired)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("expires_at", Function::new(ctx.clone(), jwt_expires_at)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("issued_at", Function::new(ctx.clone(), jwt_issued_at)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("subject", Function::new(ctx.clone(), jwt_subject)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("issuer", Function::new(ctx.clone(), jwt_issuer)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    jwt.set("audience", Function::new(ctx.clone(), jwt_audience)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("jwt", jwt)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set jwt global: {}", e)))?;

    Ok(())
}

fn decode_part(token: &str, part_index: usize) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || part_index > 1 {
        return None;
    }

    let decoded = URL_SAFE_NO_PAD.decode(parts[part_index]).ok()?;
    let json_str = String::from_utf8(decoded).ok()?;
    serde_json::from_str(&json_str).ok()
}

fn jwt_decode(token: String) -> Option<String> {
    let header = decode_part(&token, 0)?;
    let payload = decode_part(&token, 1)?;

    let result = serde_json::json!({
        "header": header,
        "payload": payload,
    });

    Some(result.to_string())
}

fn jwt_header(token: String) -> Option<String> {
    decode_part(&token, 0).map(|v| v.to_string())
}

fn jwt_payload(token: String) -> Option<String> {
    decode_part(&token, 1).map(|v| v.to_string())
}

fn get_claim(token: &str, claim: &str) -> Option<serde_json::Value> {
    let payload = decode_part(token, 1)?;
    payload.get(claim).cloned()
}

fn jwt_is_expired(token: String) -> bool {
    let exp = match get_claim(&token, "exp") {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        _ => return false,
    };

    if let Some(exp_time) = exp {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        exp_time < now
    } else {
        false
    }
}

fn jwt_expires_at(token: String) -> Option<i64> {
    match get_claim(&token, "exp") {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        _ => None,
    }
}

fn jwt_issued_at(token: String) -> Option<i64> {
    match get_claim(&token, "iat") {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        _ => None,
    }
}

fn jwt_subject(token: String) -> Option<String> {
    match get_claim(&token, "sub") {
        Some(serde_json::Value::String(s)) => Some(s),
        _ => None,
    }
}

fn jwt_issuer(token: String) -> Option<String> {
    match get_claim(&token, "iss") {
        Some(serde_json::Value::String(s)) => Some(s),
        _ => None,
    }
}

fn jwt_audience(token: String) -> Option<String> {
    match get_claim(&token, "aud") {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Array(arr)) => {
            let audiences: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            Some(audiences.join(", "))
        }
        _ => None,
    }
}
