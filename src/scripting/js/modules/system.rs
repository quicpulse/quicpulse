//! System module for JavaScript
//!
//! Provides system utilities like sleep, timestamps, and platform info.

use rquickjs::{Ctx, Object, Function};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let system = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create system object: {}", e)))?;

    system.set("sleep_ms", Function::new(ctx.clone(), sleep_ms)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    system.set("now", Function::new(ctx.clone(), now)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    system.set("now_ms", Function::new(ctx.clone(), now_ms)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    system.set("hostname", Function::new(ctx.clone(), get_hostname)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    system.set("platform", Function::new(ctx.clone(), platform)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    system.set("arch", Function::new(ctx.clone(), arch)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("system", system)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set system global: {}", e)))?;

    Ok(())
}

fn sleep_ms(ms: i32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn platform() -> String {
    std::env::consts::OS.to_string()
}

fn arch() -> String {
    std::env::consts::ARCH.to_string()
}
