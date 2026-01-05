//! Crypto module for JavaScript
//!
//! Provides cryptographic functions matching Rune's crypto module.

use rquickjs::{Ctx, Object, Function};
use sha2::{Sha256, Sha512, Digest};
use sha1::Sha1;
use hmac::{Hmac, Mac};
use uuid::Uuid;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::QuicpulseError;

type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let crypto = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create crypto object: {}", e)))?;

    // Hash functions
    crypto.set("sha256_hex", Function::new(ctx.clone(), sha256_hex)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("sha512_hex", Function::new(ctx.clone(), sha512_hex)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("sha1_hex", Function::new(ctx.clone(), sha1_hex)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("md5_hex", Function::new(ctx.clone(), md5_hex)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // HMAC functions
    crypto.set("hmac_sha256", Function::new(ctx.clone(), hmac_sha256)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("hmac_sha512", Function::new(ctx.clone(), hmac_sha512)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("hmac_sha256_base64", Function::new(ctx.clone(), hmac_sha256_base64)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // Random functions
    crypto.set("random_hex", Function::new(ctx.clone(), random_hex)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("random_bytes_base64", Function::new(ctx.clone(), random_bytes_base64)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("random_int", Function::new(ctx.clone(), random_int)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("random_string", Function::new(ctx.clone(), random_string)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // UUID functions
    crypto.set("uuid_v4", Function::new(ctx.clone(), uuid_v4)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("uuid_v7", Function::new(ctx.clone(), uuid_v7)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // Timestamp functions
    crypto.set("timestamp", Function::new(ctx.clone(), timestamp)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    crypto.set("timestamp_ms", Function::new(ctx.clone(), timestamp_ms)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("crypto", crypto)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set crypto global: {}", e)))?;

    Ok(())
}

fn sha256_hex(input: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn sha512_hex(input: String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn sha1_hex(input: String) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn md5_hex(input: String) -> String {
    let digest = md5::compute(input.as_bytes());
    hex::encode(digest.as_slice())
}

fn hmac_sha256(key: String, message: String) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn hmac_sha512(key: String, message: String) -> String {
    let mut mac = HmacSha512::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn hmac_sha256_base64(key: String, message: String) -> String {
    use base64::Engine;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

fn random_hex(length: i32) -> String {
    let byte_len = (length as usize + 1) / 2;
    let bytes: Vec<u8> = (0..byte_len).map(|_| rand::random::<u8>()).collect();
    let hex = hex::encode(&bytes);
    hex[..length as usize].to_string()
}

fn random_bytes_base64(length: i32) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = (0..length as usize).map(|_| rand::random::<u8>()).collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

fn random_int(min: i32, max: i32) -> i32 {
    rand::rng().random_range(min..=max)
}

fn random_string(length: i32) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..length as usize)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

fn uuid_v7() -> String {
    Uuid::now_v7().to_string()
}

fn timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
