//! Cryptographic module for Rune scripts
//!
//! Provides cryptographic hash functions, HMAC, and other
//! security-related utilities for scripts.

use rune::{ContextError, Module};
use rune::alloc::String as RuneString;
use sha2::{Sha256, Sha512, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;

/// Create the crypto module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("crypto")?;

    // Hash functions
    module.function("sha256_hex", sha256_hex).build()?;
    module.function("sha512_hex", sha512_hex).build()?;
    module.function("md5_hex", md5_hex).build()?;
    module.function("sha1_hex", sha1_hex).build()?;

    // HMAC functions
    module.function("hmac_sha256", hmac_sha256_hex).build()?;
    module.function("hmac_sha512", hmac_sha512_hex).build()?;
    module.function("hmac_sha256_base64", hmac_sha256_base64).build()?;

    // Random functions
    module.function("random_hex", random_hex).build()?;
    module.function("random_bytes_base64", random_bytes_base64).build()?;
    module.function("random_int", random_int).build()?;
    module.function("random_string", random_string).build()?;
    module.function("uuid_v4", uuid_v4).build()?;
    module.function("uuid_v7", uuid_v7).build()?;

    // Timestamp helpers
    module.function("timestamp", timestamp).build()?;
    module.function("timestamp_ms", timestamp_ms).build()?;

    Ok(module)
}

/// Compute SHA-256 hash and return as hex string
fn sha256_hex(input: &str) -> RuneString {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hex = hex::encode(result);
    RuneString::try_from(hex).unwrap_or_default()
}

/// Compute SHA-512 hash and return as hex string
fn sha512_hex(input: &str) -> RuneString {
    let mut hasher = Sha512::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hex = hex::encode(result);
    RuneString::try_from(hex).unwrap_or_default()
}

/// Compute MD5 hash as hex string
fn md5_hex(input: &str) -> RuneString {
    let digest = md5::compute(input.as_bytes());
    let hex = hex::encode(digest.as_slice());
    RuneString::try_from(hex).unwrap_or_default()
}

/// Generate random bytes as hex string
fn random_hex(count: i64) -> RuneString {
    use rand::RngCore;
    let count = count.max(0) as usize;
    let mut bytes = vec![0u8; count];
    rand::rng().fill_bytes(&mut bytes);
    let hex = hex::encode(&bytes);
    RuneString::try_from(hex).unwrap_or_default()
}

/// Generate a UUID v4 (random)
fn uuid_v4() -> RuneString {
    let uuid = uuid::Uuid::new_v4().to_string();
    RuneString::try_from(uuid).unwrap_or_default()
}

/// Generate a UUID v7 (time-based)
fn uuid_v7() -> RuneString {
    let uuid = uuid::Uuid::now_v7().to_string();
    RuneString::try_from(uuid).unwrap_or_default()
}

/// Compute SHA-1 hash as hex string
fn sha1_hex(input: &str) -> RuneString {
    use sha1::{Sha1, Digest as Sha1Digest};
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hex = hex::encode(result);
    RuneString::try_from(hex).unwrap_or_default()
}

/// Compute HMAC-SHA256 and return as hex string
fn hmac_sha256_hex(key: &str, message: &str) -> RuneString {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let hex = hex::encode(result.into_bytes());
    RuneString::try_from(hex).unwrap_or_default()
}

/// Compute HMAC-SHA512 and return as hex string
fn hmac_sha512_hex(key: &str, message: &str) -> RuneString {
    let mut mac = HmacSha512::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let hex = hex::encode(result.into_bytes());
    RuneString::try_from(hex).unwrap_or_default()
}

/// Compute HMAC-SHA256 and return as base64 string
fn hmac_sha256_base64(key: &str, message: &str) -> RuneString {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let b64 = STANDARD.encode(result.into_bytes());
    RuneString::try_from(b64).unwrap_or_default()
}

/// Generate random bytes as base64 string
fn random_bytes_base64(count: i64) -> RuneString {
    use rand::RngCore;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let count = count.max(0) as usize;
    let mut bytes = vec![0u8; count];
    rand::rng().fill_bytes(&mut bytes);
    let b64 = STANDARD.encode(&bytes);
    RuneString::try_from(b64).unwrap_or_default()
}

/// Generate a random integer in range [min, max]
fn random_int(min: i64, max: i64) -> i64 {
    use rand::Rng;
    let (min, max) = if min > max { (max, min) } else { (min, max) };
    let mut rng = rand::rng();
    rng.random_range(min..=max)
}

/// Generate a random alphanumeric string of given length
fn random_string(length: i64) -> RuneString {
    use rand::Rng;
    let length = length.max(0) as usize;
    let chars: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    RuneString::try_from(chars).unwrap_or_default()
}

/// Get current Unix timestamp in seconds
fn timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Get current Unix timestamp in milliseconds
fn timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256_hex("hello");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hmac_sha256() {
        let sig = hmac_sha256_hex("secret", "message");
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn test_random_int() {
        for _ in 0..100 {
            let n = random_int(1, 10);
            assert!(n >= 1 && n <= 10);
        }
    }

    #[test]
    fn test_random_string() {
        let s = random_string(16);
        assert_eq!(s.len(), 16);
    }
}
