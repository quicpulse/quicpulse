//! Encoding module for Rune scripts
//!
//! Provides base64, URL encoding, hex encoding, and other
//! encoding/decoding utilities for scripts.

use rune::{ContextError, Module};
use rune::alloc::String as RuneString;
use base64::Engine;

/// Create the encoding module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("encoding")?;

    // Base64
    module.function("base64_encode", base64_encode).build()?;
    module.function("base64_decode", base64_decode).build()?;

    // URL encoding
    module.function("url_encode", url_encode).build()?;
    module.function("url_decode", url_decode).build()?;

    // Hex encoding
    module.function("hex_encode", hex_encode).build()?;
    module.function("hex_decode", hex_decode).build()?;

    // HTML entities
    module.function("html_escape", html_escape).build()?;

    Ok(module)
}

/// Encode string to base64
fn base64_encode(input: &str) -> RuneString {
    let encoded = base64::engine::general_purpose::STANDARD.encode(input.as_bytes());
    RuneString::try_from(encoded).unwrap_or_default()
}

/// Decode base64 string
fn base64_decode(input: &str) -> RuneString {
    match base64::engine::general_purpose::STANDARD.decode(input.trim()) {
        Ok(bytes) => match std::string::String::from_utf8(bytes) {
            Ok(s) => RuneString::try_from(s).unwrap_or_default(),
            Err(_) => RuneString::new(),
        },
        Err(_) => RuneString::new(),
    }
}

/// URL-encode a string (percent encoding)
fn url_encode(input: &str) -> RuneString {
    let encoded = urlencoding::encode(input).into_owned();
    RuneString::try_from(encoded).unwrap_or_default()
}

/// URL-decode a string
fn url_decode(input: &str) -> RuneString {
    match urlencoding::decode(input) {
        Ok(s) => RuneString::try_from(s.into_owned()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Hex-encode bytes from a string
fn hex_encode(input: &str) -> RuneString {
    let encoded = hex::encode(input.as_bytes());
    RuneString::try_from(encoded).unwrap_or_default()
}

/// Hex-decode to a string
fn hex_decode(input: &str) -> RuneString {
    match hex::decode(input.trim()) {
        Ok(bytes) => match std::string::String::from_utf8(bytes) {
            Ok(s) => RuneString::try_from(s).unwrap_or_default(),
            Err(_) => RuneString::new(),
        },
        Err(_) => RuneString::new(),
    }
}

/// Escape HTML entities
fn html_escape(input: &str) -> RuneString {
    let escaped = input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    RuneString::try_from(escaped).unwrap_or_default()
}
