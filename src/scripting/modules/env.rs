//! Environment module for Rune scripts
//!
//! Provides access to environment variables and system information
//! in a controlled manner.

use rune::{ContextError, Module};
use rune::alloc::String as RuneString;

/// Create the env module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("env")?;

    // Environment variable access
    module.function("get", env_get).build()?;
    module.function("get_or", env_get_or).build()?;
    module.function("has", env_has).build()?;

    // Safe environment info
    module.function("os", env_os).build()?;
    module.function("arch", env_arch).build()?;

    // Time utilities
    module.function("now", env_now).build()?;
    module.function("now_millis", env_now_millis).build()?;
    module.function("now_iso", env_now_iso).build()?;

    Ok(module)
}

// List of allowed environment variables for security
const ALLOWED_ENV_VARS: &[&str] = &[
    "HOME",
    "USER",
    "LANG",
    "LC_ALL",
    "TZ",
    "SHELL",
    "TERM",
    "PATH",
    "PWD",
    "TMPDIR",
    "TEMP",
    "TMP",
    // HTTP-related
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    // QuicPulse-specific (allow custom prefix)
    "QUICPULSE_",
    "QP_",
];

/// Check if an environment variable is in the allowed list
/// Used to prevent leaking sensitive environment variables like AWS_SECRET_ACCESS_KEY
pub fn is_allowed_env_var(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    for allowed in ALLOWED_ENV_VARS {
        if allowed.ends_with('_') {
            // Prefix match
            if name_upper.starts_with(allowed) {
                return true;
            }
        } else {
            // Exact match
            if name_upper == *allowed {
                return true;
            }
        }
    }
    false
}

/// Get environment variable (returns empty string if not found or not allowed)
fn env_get(name: &str) -> RuneString {
    if !is_allowed_env_var(name) {
        return RuneString::new();
    }
    match std::env::var(name) {
        Ok(val) => RuneString::try_from(val).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Get environment variable with default
fn env_get_or(name: &str, default: &str) -> RuneString {
    if !is_allowed_env_var(name) {
        return RuneString::try_from(default).unwrap_or_default();
    }
    match std::env::var(name) {
        Ok(val) => RuneString::try_from(val).unwrap_or_default(),
        Err(_) => RuneString::try_from(default).unwrap_or_default(),
    }
}

/// Check if environment variable exists and is allowed
fn env_has(name: &str) -> bool {
    if !is_allowed_env_var(name) {
        return false;
    }
    std::env::var(name).is_ok()
}

/// Get operating system name
fn env_os() -> RuneString {
    RuneString::try_from(std::env::consts::OS).unwrap_or_default()
}

/// Get CPU architecture
fn env_arch() -> RuneString {
    RuneString::try_from(std::env::consts::ARCH).unwrap_or_default()
}

/// Get current timestamp as Unix epoch seconds
fn env_now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Get current timestamp as Unix epoch milliseconds
fn env_now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Get current timestamp as ISO 8601 string
fn env_now_iso() -> RuneString {
    let iso = chrono::Utc::now().to_rfc3339();
    RuneString::try_from(iso).unwrap_or_default()
}
