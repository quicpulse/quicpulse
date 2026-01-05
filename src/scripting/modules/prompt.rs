//! Prompt module for interactive user input
//!
//! Allows scripts to pause and request input from the user,
//! useful for MFA/OTP flows, confirmations, and dynamic input.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use dialoguer::{Input, Password, Confirm, Select};
use std::io::{self, Write};

/// Create the prompt module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("prompt")?;

    // Text input
    module.function("text", prompt_text).build()?;
    module.function("text_default", prompt_text_default).build()?;

    // Password/hidden input
    module.function("password", prompt_password).build()?;

    // Confirmation
    module.function("confirm", prompt_confirm).build()?;
    module.function("confirm_default", prompt_confirm_default).build()?;

    // Selection from list
    module.function("select", prompt_select).build()?;

    Ok(module)
}

/// Prompt for text input
fn prompt_text(message: &str) -> RuneString {
    // Ensure we're writing to stderr to not interfere with JSON output
    eprint!("{}", message);
    io::stderr().flush().ok();

    match Input::<String>::new()
        .with_prompt(message)
        .interact_text()
    {
        Ok(input) => RuneString::try_from(input).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Prompt for text input with a default value
fn prompt_text_default(message: &str, default: &str) -> RuneString {
    match Input::<String>::new()
        .with_prompt(message)
        .default(default.to_string())
        .interact_text()
    {
        Ok(input) => RuneString::try_from(input).unwrap_or_default(),
        Err(_) => RuneString::try_from(default.to_string()).unwrap_or_default(),
    }
}

/// Prompt for password (hidden input)
/// Writes prompt to stderr to avoid corrupting stdout output (e.g., JSON)
fn prompt_password(message: &str) -> RuneString {
    // Use interact_on to write to stderr, not stdout
    // This prevents prompt from appearing in piped output
    // Use dialoguer's re-exported console to avoid version mismatch
    let term = dialoguer::console::Term::stderr();

    match Password::new()
        .with_prompt(message)
        .interact_on(&term)
    {
        Ok(input) => RuneString::try_from(input).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Prompt for yes/no confirmation
fn prompt_confirm(message: &str) -> bool {
    Confirm::new()
        .with_prompt(message)
        .interact()
        .unwrap_or(false)
}

/// Prompt for yes/no confirmation with default
fn prompt_confirm_default(message: &str, default: bool) -> bool {
    Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()
        .unwrap_or(default)
}

/// Prompt user to select from a list of options
/// Returns the index of the selected item (0-based)
fn prompt_select(message: &str, options: &str) -> i64 {
    // Parse options from comma-separated string
    let items: Vec<&str> = options.split(',').map(|s| s.trim()).collect();

    if items.is_empty() {
        return -1;
    }

    match Select::new()
        .with_prompt(message)
        .items(&items)
        .default(0)
        .interact()
    {
        Ok(index) => index as i64,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    // Interactive prompts can't be easily tested in automated tests
    // Manual testing is required for these functions
}
