//! JWT module for token parsing and debugging
//!
//! Provides functions to decode and inspect JWT tokens without verification.
//! Useful for debugging authentication flows.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value as JsonValue;

/// Create the JWT module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("jwt")?;

    // Core functions
    module.function("decode", decode).build()?;
    module.function("decode_header", decode_header).build()?;
    module.function("decode_payload", decode_payload).build()?;

    // Convenience accessors
    module.function("get_claim", get_claim).build()?;
    module.function("get_exp", get_exp).build()?;
    module.function("get_iat", get_iat).build()?;
    module.function("get_sub", get_sub).build()?;
    module.function("get_iss", get_iss).build()?;
    module.function("get_aud", get_aud).build()?;

    // Validation helpers (no cryptographic verification)
    module.function("is_expired", is_expired).build()?;
    module.function("expires_in", expires_in).build()?;
    module.function("parts_count", parts_count).build()?;
    module.function("is_valid_format", is_valid_format).build()?;

    Ok(module)
}

/// Decode a JWT and return the full payload as JSON string
fn decode(token: &str) -> RuneString {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return RuneString::try_from("{}").unwrap_or_default();
    }

    match decode_base64_json(parts[1]) {
        Some(json) => RuneString::try_from(json.to_string()).unwrap_or_default(),
        None => RuneString::try_from("{}").unwrap_or_default(),
    }
}

/// Decode just the header portion
fn decode_header(token: &str) -> RuneString {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.is_empty() {
        return RuneString::try_from("{}").unwrap_or_default();
    }

    match decode_base64_json(parts[0]) {
        Some(json) => RuneString::try_from(json.to_string()).unwrap_or_default(),
        None => RuneString::try_from("{}").unwrap_or_default(),
    }
}

/// Decode just the payload portion (alias for decode)
fn decode_payload(token: &str) -> RuneString {
    decode(token)
}

/// Get a specific claim from the payload
fn get_claim(token: &str, claim: &str) -> RuneString {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return RuneString::new();
    }

    match decode_base64_json(parts[1]) {
        Some(json) => {
            if let Some(value) = json.get(claim) {
                match value {
                    JsonValue::String(s) => RuneString::try_from(s.clone()).unwrap_or_default(),
                    other => RuneString::try_from(other.to_string()).unwrap_or_default(),
                }
            } else {
                RuneString::new()
            }
        }
        None => RuneString::new(),
    }
}

/// Get the expiration timestamp (exp claim)
fn get_exp(token: &str) -> i64 {
    get_numeric_claim(token, "exp")
}

/// Get the issued-at timestamp (iat claim)
fn get_iat(token: &str) -> i64 {
    get_numeric_claim(token, "iat")
}

/// Get the subject (sub claim)
fn get_sub(token: &str) -> RuneString {
    get_claim(token, "sub")
}

/// Get the issuer (iss claim)
fn get_iss(token: &str) -> RuneString {
    get_claim(token, "iss")
}

/// Get the audience (aud claim)
fn get_aud(token: &str) -> RuneString {
    get_claim(token, "aud")
}

/// Check if the token is expired
fn is_expired(token: &str) -> bool {
    let exp = get_exp(token);
    if exp == 0 {
        return false; // No exp claim, can't determine
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    exp < now
}

/// Get seconds until expiration (negative if expired)
fn expires_in(token: &str) -> i64 {
    let exp = get_exp(token);
    if exp == 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    exp - now
}

/// Count the number of parts in the token (should be 3 for valid JWT)
fn parts_count(token: &str) -> i64 {
    token.split('.').count() as i64
}

/// Check if the token has valid JWT format (3 base64 parts)
fn is_valid_format(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    // Try to decode each part
    decode_base64_json(parts[0]).is_some() && decode_base64_json(parts[1]).is_some()
}

// Helper functions

fn decode_base64_json(input: &str) -> Option<JsonValue> {
    // Try URL-safe base64 first, then standard
    let decoded = URL_SAFE_NO_PAD.decode(input)
        .or_else(|_| {
            // Try with padding
            let padded = match input.len() % 4 {
                2 => format!("{}==", input),
                3 => format!("{}=", input),
                _ => input.to_string(),
            };
            URL_SAFE_NO_PAD.decode(&padded)
        })
        .ok()?;

    let json_str = String::from_utf8(decoded).ok()?;
    serde_json::from_str(&json_str).ok()
}

fn get_numeric_claim(token: &str, claim: &str) -> i64 {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return 0;
    }

    match decode_base64_json(parts[1]) {
        Some(json) => {
            if let Some(value) = json.get(claim) {
                match value {
                    JsonValue::Number(n) => n.as_i64().unwrap_or(0),
                    _ => 0,
                }
            } else {
                0
            }
        }
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test JWT: {"alg":"HS256","typ":"JWT"}.{"sub":"1234567890","name":"John Doe","iat":1516239022,"exp":9999999999}
    const TEST_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjk5OTk5OTk5OTl9.Vg30mf-jGc7xqAGbKlhvTqJdNeXxMjS_CBQJ3WA3Dds";

    #[test]
    fn test_decode() {
        let payload = decode(TEST_TOKEN);
        assert!(payload.contains("1234567890"));
        assert!(payload.contains("John Doe"));
    }

    #[test]
    fn test_get_claim() {
        let sub = get_sub(TEST_TOKEN);
        assert_eq!(sub.as_str(), "1234567890");
    }

    #[test]
    fn test_is_expired() {
        assert!(!is_expired(TEST_TOKEN)); // exp is in far future
    }

    #[test]
    fn test_is_valid_format() {
        assert!(is_valid_format(TEST_TOKEN));
        assert!(!is_valid_format("not.a.jwt"));
        assert!(!is_valid_format("invalid"));
    }

    #[test]
    fn test_parts_count() {
        assert_eq!(parts_count(TEST_TOKEN), 3);
        assert_eq!(parts_count("a.b"), 2);
    }
}
