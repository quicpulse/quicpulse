//! HTTP module for JavaScript
//!
//! Provides HTTP status code constants and helper functions.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let http = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create http object: {}", e)))?;

    // Status code constants
    http.set("OK", 200i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("CREATED", 201i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("ACCEPTED", 202i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("NO_CONTENT", 204i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("MOVED_PERMANENTLY", 301i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("FOUND", 302i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("NOT_MODIFIED", 304i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("BAD_REQUEST", 400i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("UNAUTHORIZED", 401i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("FORBIDDEN", 403i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("NOT_FOUND", 404i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("METHOD_NOT_ALLOWED", 405i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("CONFLICT", 409i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("GONE", 410i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("UNPROCESSABLE_ENTITY", 422i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("TOO_MANY_REQUESTS", 429i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("INTERNAL_SERVER_ERROR", 500i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("BAD_GATEWAY", 502i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("SERVICE_UNAVAILABLE", 503i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("GATEWAY_TIMEOUT", 504i32).map_err(|e| QuicpulseError::Script(e.to_string()))?;

    // Helper functions
    http.set("is_success", Function::new(ctx.clone(), is_success)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("is_redirect", Function::new(ctx.clone(), is_redirect)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("is_client_error", Function::new(ctx.clone(), is_client_error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("is_server_error", Function::new(ctx.clone(), is_server_error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    http.set("is_error", Function::new(ctx.clone(), is_error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("http", http)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set http global: {}", e)))?;

    Ok(())
}

fn is_success(status: i32) -> bool {
    (200..300).contains(&status)
}

fn is_redirect(status: i32) -> bool {
    (300..400).contains(&status)
}

fn is_client_error(status: i32) -> bool {
    (400..500).contains(&status)
}

fn is_server_error(status: i32) -> bool {
    (500..600).contains(&status)
}

fn is_error(status: i32) -> bool {
    status >= 400
}
