//! Console module for JavaScript
//!
//! Provides logging functions that output to stderr.

use rquickjs::{Ctx, Object, Function};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let console = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create console object: {}", e)))?;

    console.set("log", Function::new(ctx.clone(), log)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("info", Function::new(ctx.clone(), info)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("warn", Function::new(ctx.clone(), warn)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("error", Function::new(ctx.clone(), error)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("debug", Function::new(ctx.clone(), debug)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("trace", Function::new(ctx.clone(), trace)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    console.set("success", Function::new(ctx.clone(), success)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("console", console)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set console global: {}", e)))?;

    Ok(())
}

fn log(message: String) {
    eprintln!("[LOG] {}", message);
}

fn info(message: String) {
    eprintln!("[INFO] {}", message);
}

fn warn(message: String) {
    eprintln!("[WARN] {}", message);
}

fn error(message: String) {
    eprintln!("[ERROR] {}", message);
}

fn debug(message: String) {
    eprintln!("[DEBUG] {}", message);
}

fn trace(message: String) {
    eprintln!("[TRACE] {}", message);
}

fn success(message: String) {
    eprintln!("[SUCCESS] {}", message);
}
