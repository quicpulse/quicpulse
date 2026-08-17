//! JavaScript module registrations
//!
//! Registers all built-in modules to match Rune module parity.

mod assert;
mod console;
mod cookie;
mod crypto;
mod date;
mod encoding;
mod env;
mod faker;
mod fs;
mod http;
mod json;
mod jwt;
mod prompt;
mod regex;
mod schema;
mod store;
mod system;
mod url;
mod xml;

use crate::errors::QuicpulseError;
use rquickjs::Ctx;

/// Register all built-in modules
pub fn register_all(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    http::register(ctx)?;
    crypto::register(ctx)?;
    json::register(ctx)?;
    console::register(ctx)?;
    assert::register(ctx)?;
    store::register(ctx)?;
    encoding::register(ctx)?;
    env::register(ctx)?;
    system::register(ctx)?;
    url::register(ctx)?;
    regex::register(ctx)?;
    date::register(ctx)?;
    faker::register(ctx)?;
    jwt::register(ctx)?;
    fs::register(ctx)?;
    cookie::register(ctx)?;
    schema::register(ctx)?;
    xml::register(ctx)?;
    prompt::register(ctx)?;

    Ok(())
}
