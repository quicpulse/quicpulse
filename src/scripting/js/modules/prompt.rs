//! Prompt module for JavaScript
//!
//! Provides interactive user input capabilities.
//! Note: This module is only functional in interactive mode.

use rquickjs::{Ctx, Object, Function};
use std::io::{self, Write};
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let prompt = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create prompt object: {}", e)))?;

    prompt.set("input", Function::new(ctx.clone(), prompt_input)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    prompt.set("password", Function::new(ctx.clone(), prompt_password)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    prompt.set("confirm", Function::new(ctx.clone(), prompt_confirm)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    prompt.set("select", Function::new(ctx.clone(), prompt_select)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("prompt", prompt)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set prompt global: {}", e)))?;

    Ok(())
}

/// Check if we're in an interactive terminal
fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}

/// Prompt for text input
fn prompt_input(message: String, default: Option<String>) -> Option<String> {
    if !is_interactive() {
        return default;
    }

    let prompt_text = if let Some(ref def) = default {
        format!("{} [{}]: ", message, def)
    } else {
        format!("{}: ", message)
    };

    print!("{}", prompt_text);
    io::stdout().flush().ok()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let input = input.trim().to_string();

    if input.is_empty() {
        default
    } else {
        Some(input)
    }
}

/// Prompt for password input (hidden)
fn prompt_password(message: String) -> Option<String> {
    if !is_interactive() {
        return None;
    }

    print!("{}: ", message);
    io::stdout().flush().ok()?;

    // Try to use rpassword for hidden input
    rpassword::read_password().ok()
}

/// Prompt for yes/no confirmation
fn prompt_confirm(message: String, default: Option<bool>) -> bool {
    if !is_interactive() {
        return default.unwrap_or(false);
    }

    let prompt_text = match default {
        Some(true) => format!("{} [Y/n]: ", message),
        Some(false) => format!("{} [y/N]: ", message),
        None => format!("{} [y/n]: ", message),
    };

    print!("{}", prompt_text);
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default.unwrap_or(false);
    }

    let input = input.trim().to_lowercase();

    if input.is_empty() {
        default.unwrap_or(false)
    } else {
        matches!(input.as_str(), "y" | "yes" | "true" | "1")
    }
}

/// Prompt user to select from a list of options
/// options_json should be a JSON array of strings
/// Returns the selected option or None
fn prompt_select(message: String, options_json: String) -> Option<String> {
    if !is_interactive() {
        return None;
    }

    let options: Vec<String> = match serde_json::from_str(&options_json) {
        Ok(o) => o,
        Err(_) => return None,
    };

    if options.is_empty() {
        return None;
    }

    println!("{}:", message);
    for (i, option) in options.iter().enumerate() {
        println!("  {}. {}", i + 1, option);
    }

    print!("Enter number (1-{}): ", options.len());
    io::stdout().flush().ok()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;

    input.trim().parse::<usize>().ok()
        .filter(|&n| n >= 1 && n <= options.len())
        .map(|n| options[n - 1].clone())
}
