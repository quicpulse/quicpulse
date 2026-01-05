//! Env module for JavaScript
//!
//! Provides safe access to environment variables.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

/// Allowed environment variables (security: limit exposure)
const ALLOWED_ENV_VARS: &[&str] = &[
    "HOME", "USER", "SHELL", "TERM", "LANG", "PATH",
    "PWD", "OLDPWD", "HOSTNAME", "LOGNAME",
    "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME",
];

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let env = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create env object: {}", e)))?;

    env.set("get", Function::new(ctx.clone(), env_get)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    env.set("get_or", Function::new(ctx.clone(), env_get_or)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    env.set("has", Function::new(ctx.clone(), env_has)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("env", env)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set env global: {}", e)))?;

    Ok(())
}

fn is_allowed(key: &str) -> bool {
    // Allow QUICPULSE_* variables and standard ones
    key.starts_with("QUICPULSE_") || ALLOWED_ENV_VARS.contains(&key)
}

fn env_get(key: String) -> Option<String> {
    if is_allowed(&key) {
        std::env::var(&key).ok()
    } else {
        None
    }
}

fn env_get_or(key: String, default: String) -> String {
    if is_allowed(&key) {
        std::env::var(&key).unwrap_or(default)
    } else {
        default
    }
}

fn env_has(key: String) -> bool {
    if is_allowed(&key) {
        std::env::var(&key).is_ok()
    } else {
        false
    }
}
