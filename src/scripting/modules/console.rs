//! Console module for structured logging
//!
//! Provides logging functions that output to stderr with proper formatting,
//! avoiding interference with JSON output on stdout.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Verbosity level (0 = normal, 1 = verbose, 2 = debug)
static VERBOSITY: AtomicU8 = AtomicU8::new(0);

/// Whether colors are enabled
static COLORS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Create the console module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("console")?;

    // Logging levels
    module.function("log", log_info).build()?;
    module.function("info", log_info).build()?;
    module.function("warn", log_warn).build()?;
    module.function("error", log_error).build()?;
    module.function("debug", log_debug).build()?;
    module.function("trace", log_trace).build()?;

    // Success/failure
    module.function("success", log_success).build()?;
    module.function("fail", log_fail).build()?;

    // Formatting
    module.function("print", print_raw).build()?;
    module.function("println", println_raw).build()?;
    module.function("newline", newline).build()?;
    module.function("hr", horizontal_rule).build()?;

    // JSON output
    module.function("json", print_json).build()?;
    module.function("table", print_table).build()?;

    // Timing
    module.function("time", time_start).build()?;
    module.function("time_end", time_end).build()?;

    // Grouping
    module.function("group", group_start).build()?;
    module.function("group_end", group_end).build()?;

    // Progress
    module.function("progress", log_progress).build()?;

    Ok(module)
}

/// Set verbosity level
pub fn set_verbosity(level: u8) {
    VERBOSITY.store(level, Ordering::Relaxed);
}

/// Enable or disable colors
pub fn set_colors(enabled: bool) {
    COLORS_ENABLED.store(enabled, Ordering::Relaxed);
}

// ANSI color codes
fn color(code: &str) -> &str {
    if COLORS_ENABLED.load(Ordering::Relaxed) {
        code
    } else {
        ""
    }
}

fn reset() -> &'static str {
    if COLORS_ENABLED.load(Ordering::Relaxed) {
        "\x1b[0m"
    } else {
        ""
    }
}

/// Log info message
fn log_info(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}[INFO]{} {}",
        color("\x1b[36m"), // cyan
        reset(),
        message
    ).ok();
}

/// Log warning message
fn log_warn(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}[WARN]{} {}",
        color("\x1b[33m"), // yellow
        reset(),
        message
    ).ok();
}

/// Log error message
fn log_error(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}[ERROR]{} {}",
        color("\x1b[31m"), // red
        reset(),
        message
    ).ok();
}

/// Log debug message (only if verbosity >= 1)
fn log_debug(message: &str) {
    if VERBOSITY.load(Ordering::Relaxed) >= 1 {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(
            handle,
            "{}[DEBUG]{} {}",
            color("\x1b[35m"), // magenta
            reset(),
            message
        ).ok();
    }
}

/// Log trace message (only if verbosity >= 2)
fn log_trace(message: &str) {
    if VERBOSITY.load(Ordering::Relaxed) >= 2 {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(
            handle,
            "{}[TRACE]{} {}",
            color("\x1b[90m"), // gray
            reset(),
            message
        ).ok();
    }
}

/// Log success message
fn log_success(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}✓{} {}",
        color("\x1b[32m"), // green
        reset(),
        message
    ).ok();
}

/// Log failure message
fn log_fail(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}✗{} {}",
        color("\x1b[31m"), // red
        reset(),
        message
    ).ok();
}

/// Print raw text to stderr (no newline)
fn print_raw(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    write!(handle, "{}", message).ok();
    handle.flush().ok();
}

/// Print text with newline to stderr
fn println_raw(message: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(handle, "{}", message).ok();
}

/// Print a newline
fn newline() {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(handle).ok();
}

/// Print a horizontal rule
fn horizontal_rule() {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}────────────────────────────────────────{}",
        color("\x1b[90m"),
        reset()
    ).ok();
}

/// Pretty-print JSON to stderr
fn print_json(json_str: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    // Try to parse and pretty-print
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(value) => {
            if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                writeln!(handle, "{}", pretty).ok();
            } else {
                writeln!(handle, "{}", json_str).ok();
            }
        }
        Err(_) => {
            writeln!(handle, "{}", json_str).ok();
        }
    }
}

/// Print a simple table from JSON array of objects
fn print_table(json_str: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(serde_json::Value::Array(arr)) => {
            if arr.is_empty() {
                writeln!(handle, "(empty)").ok();
                return;
            }

            // Get column headers from first object
            let headers: Vec<String> = if let Some(serde_json::Value::Object(obj)) = arr.first() {
                obj.keys().cloned().collect()
            } else {
                writeln!(handle, "{}", json_str).ok();
                return;
            };

            // Print headers
            writeln!(
                handle,
                "{}{}{}",
                color("\x1b[1m"),
                headers.join("\t"),
                reset()
            ).ok();

            // Print rows
            for item in arr {
                if let serde_json::Value::Object(obj) = item {
                    let row: Vec<String> = headers.iter()
                        .map(|h| {
                            obj.get(h)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default()
                        })
                        .collect();
                    writeln!(handle, "{}", row.join("\t")).ok();
                }
            }
        }
        _ => {
            writeln!(handle, "{}", json_str).ok();
        }
    }
}

// Timing storage using DashMap for non-blocking concurrent access
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::Instant;

/// Timer storage using DashMap for lock-free concurrent access
static TIMERS: Lazy<DashMap<String, Instant>> = Lazy::new(DashMap::new);

/// Start a timer
fn time_start(label: &str) {
    TIMERS.insert(label.to_string(), Instant::now());

    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}⏱ {}: timer started{}",
        color("\x1b[90m"),
        label,
        reset()
    ).ok();
}

/// End a timer and print elapsed time
fn time_end(label: &str) {
    let elapsed = TIMERS.remove(label).map(|(_, start)| start.elapsed());

    let stderr = io::stderr();
    let mut handle = stderr.lock();

    if let Some(duration) = elapsed {
        writeln!(
            handle,
            "{}⏱ {}: {:.2}ms{}",
            color("\x1b[90m"),
            label,
            duration.as_secs_f64() * 1000.0,
            reset()
        ).ok();
    } else {
        writeln!(
            handle,
            "{}⏱ {}: timer not found{}",
            color("\x1b[90m"),
            label,
            reset()
        ).ok();
    }
}

// Group indentation level
static GROUP_LEVEL: AtomicU8 = AtomicU8::new(0);

/// Start a group
fn group_start(label: &str) {
    let level = GROUP_LEVEL.fetch_add(1, Ordering::Relaxed);
    let indent = "  ".repeat(level as usize);

    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "{}{}▶ {}{}",
        indent,
        color("\x1b[1m"),
        label,
        reset()
    ).ok();
}

/// End a group
fn group_end() {
    GROUP_LEVEL.fetch_sub(1, Ordering::Relaxed);
}

/// Log progress (percentage)
fn log_progress(message: &str, percent: i64) {
    let percent = percent.clamp(0, 100);
    let filled = (percent as usize) / 5;
    let empty = 20 - filled;

    let bar = format!(
        "[{}{}] {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    );

    let stderr = io::stderr();
    let mut handle = stderr.lock();
    write!(
        handle,
        "\r{}{} {}{}",
        color("\x1b[36m"),
        bar,
        message,
        reset()
    ).ok();
    handle.flush().ok();

    if percent >= 100 {
        writeln!(handle).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_levels() {
        set_verbosity(0);
        // debug and trace shouldn't output at level 0
        log_debug("This should not appear");

        set_verbosity(1);
        log_debug("This should appear at level 1");

        set_verbosity(2);
        log_trace("This should appear at level 2");
    }

    #[test]
    fn test_timer() {
        time_start("test_timer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        time_end("test_timer");
    }
}
