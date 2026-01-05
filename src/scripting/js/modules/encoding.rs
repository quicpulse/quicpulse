//! Encoding module for JavaScript
//!
//! Provides base64, hex, and URL encoding/decoding functions.

use rquickjs::{Ctx, Object, Function};
use base64::Engine;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let encoding = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create encoding object: {}", e)))?;

    encoding.set("base64_encode", Function::new(ctx.clone(), base64_encode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    encoding.set("base64_decode", Function::new(ctx.clone(), base64_decode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    encoding.set("hex_encode", Function::new(ctx.clone(), hex_encode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    encoding.set("hex_decode", Function::new(ctx.clone(), hex_decode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    encoding.set("url_encode", Function::new(ctx.clone(), url_encode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    encoding.set("url_decode", Function::new(ctx.clone(), url_decode)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("encoding", encoding)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set encoding global: {}", e)))?;

    Ok(())
}

fn base64_encode(input: String) -> String {
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

fn base64_decode(input: String) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(&input)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn hex_encode(input: String) -> String {
    hex::encode(input.as_bytes())
}

fn hex_decode(input: String) -> Option<String> {
    hex::decode(&input)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn url_encode(input: String) -> String {
    urlencoding::encode(&input).into_owned()
}

fn url_decode(input: String) -> Option<String> {
    urlencoding::decode(&input).ok().map(|s| s.into_owned())
}
