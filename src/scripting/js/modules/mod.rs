//! JavaScript module registrations
//!
//! Registers all built-in modules to match Rune module parity.

mod http;
mod crypto;
mod json;
mod console;
mod assert;
mod store;
mod encoding;
mod env;
mod system;
mod url;
mod regex;
mod date;
mod faker;
mod jwt;
mod fs;
mod cookie;
mod schema;
mod xml;
mod prompt;

use rquickjs::Ctx;
use crate::errors::QuicpulseError;

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
