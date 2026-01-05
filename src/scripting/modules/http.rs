//! HTTP module for Rune scripts
//!
//! Provides HTTP-related types and functions for manipulating
//! requests and responses in scripts.

use rune::{ContextError, Module};

/// Create the http module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("http")?;

    // Status code constants
    module.constant("OK", 200i64).build()?;
    module.constant("CREATED", 201i64).build()?;
    module.constant("ACCEPTED", 202i64).build()?;
    module.constant("NO_CONTENT", 204i64).build()?;
    module.constant("MOVED_PERMANENTLY", 301i64).build()?;
    module.constant("FOUND", 302i64).build()?;
    module.constant("NOT_MODIFIED", 304i64).build()?;
    module.constant("BAD_REQUEST", 400i64).build()?;
    module.constant("UNAUTHORIZED", 401i64).build()?;
    module.constant("FORBIDDEN", 403i64).build()?;
    module.constant("NOT_FOUND", 404i64).build()?;
    module.constant("METHOD_NOT_ALLOWED", 405i64).build()?;
    module.constant("CONFLICT", 409i64).build()?;
    module.constant("GONE", 410i64).build()?;
    module.constant("UNPROCESSABLE_ENTITY", 422i64).build()?;
    module.constant("TOO_MANY_REQUESTS", 429i64).build()?;
    module.constant("INTERNAL_SERVER_ERROR", 500i64).build()?;
    module.constant("BAD_GATEWAY", 502i64).build()?;
    module.constant("SERVICE_UNAVAILABLE", 503i64).build()?;
    module.constant("GATEWAY_TIMEOUT", 504i64).build()?;

    // Status code helper functions
    module.function("is_success", is_success).build()?;
    module.function("is_redirect", is_redirect).build()?;
    module.function("is_client_error", is_client_error).build()?;
    module.function("is_server_error", is_server_error).build()?;
    module.function("is_error", is_error).build()?;

    Ok(module)
}

/// Check if status code indicates success (2xx)
fn is_success(status: i64) -> bool {
    status >= 200 && status < 300
}

/// Check if status code indicates redirect (3xx)
fn is_redirect(status: i64) -> bool {
    status >= 300 && status < 400
}

/// Check if status code indicates client error (4xx)
fn is_client_error(status: i64) -> bool {
    status >= 400 && status < 500
}

/// Check if status code indicates server error (5xx)
fn is_server_error(status: i64) -> bool {
    status >= 500 && status < 600
}

/// Check if status code indicates any error (4xx or 5xx)
fn is_error(status: i64) -> bool {
    status >= 400
}
