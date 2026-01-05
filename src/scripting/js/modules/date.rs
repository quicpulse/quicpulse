//! Date module for JavaScript
//!
//! Provides date/time utilities.

use rquickjs::{Ctx, Object, Function};
use chrono::{DateTime, Utc, Local, NaiveDateTime, TimeZone};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let date = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create date object: {}", e)))?;

    date.set("now", Function::new(ctx.clone(), date_now)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("now_utc", Function::new(ctx.clone(), date_now_utc)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("format", Function::new(ctx.clone(), date_format)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("parse", Function::new(ctx.clone(), date_parse)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("timestamp", Function::new(ctx.clone(), date_timestamp)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("from_timestamp", Function::new(ctx.clone(), date_from_timestamp)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("iso", Function::new(ctx.clone(), date_iso)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    date.set("rfc2822", Function::new(ctx.clone(), date_rfc2822)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("date", date)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set date global: {}", e)))?;

    Ok(())
}

fn date_now() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn date_now_utc() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn date_format(timestamp: i64, format: String) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format(&format).to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string())
}

fn date_parse(date_str: String, format: String) -> Option<i64> {
    NaiveDateTime::parse_from_str(&date_str, &format)
        .ok()
        .map(|dt| dt.and_utc().timestamp())
}

fn date_timestamp() -> i64 {
    Utc::now().timestamp()
}

fn date_from_timestamp(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string())
}

fn date_iso() -> String {
    Utc::now().to_rfc3339()
}

fn date_rfc2822() -> String {
    Utc::now().to_rfc2822()
}
