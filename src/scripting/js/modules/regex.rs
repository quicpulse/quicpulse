//! Regex module for JavaScript
//!
//! Provides regular expression functions.

use rquickjs::{Ctx, Object, Function};
use regex::Regex;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let regex_obj = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create regex object: {}", e)))?;

    regex_obj.set("test", Function::new(ctx.clone(), regex_test)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("match", Function::new(ctx.clone(), regex_match)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("find_all", Function::new(ctx.clone(), regex_find_all)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("replace", Function::new(ctx.clone(), regex_replace)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("replace_all", Function::new(ctx.clone(), regex_replace_all)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("split", Function::new(ctx.clone(), regex_split)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    regex_obj.set("escape", Function::new(ctx.clone(), regex_escape)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("regex", regex_obj)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set regex global: {}", e)))?;

    Ok(())
}

fn regex_test(pattern: String, text: String) -> bool {
    Regex::new(&pattern).map(|re| re.is_match(&text)).unwrap_or(false)
}

fn regex_match(pattern: String, text: String) -> Option<String> {
    Regex::new(&pattern).ok().and_then(|re| {
        re.find(&text).map(|m| m.as_str().to_string())
    })
}

fn regex_find_all(pattern: String, text: String) -> String {
    let matches: Vec<String> = Regex::new(&pattern)
        .map(|re| re.find_iter(&text).map(|m| m.as_str().to_string()).collect())
        .unwrap_or_default();
    serde_json::to_string(&matches).unwrap_or_else(|_| "[]".to_string())
}

fn regex_replace(pattern: String, text: String, replacement: String) -> String {
    Regex::new(&pattern)
        .map(|re| re.replace(&text, replacement.as_str()).to_string())
        .unwrap_or(text)
}

fn regex_replace_all(pattern: String, text: String, replacement: String) -> String {
    Regex::new(&pattern)
        .map(|re| re.replace_all(&text, replacement.as_str()).to_string())
        .unwrap_or(text)
}

fn regex_split(pattern: String, text: String) -> String {
    let parts: Vec<String> = Regex::new(&pattern)
        .map(|re| re.split(&text).map(|s| s.to_string()).collect())
        .unwrap_or_else(|_| vec![text.clone()]);
    serde_json::to_string(&parts).unwrap_or_else(|_| "[]".to_string())
}

fn regex_escape(text: String) -> String {
    regex::escape(&text)
}
