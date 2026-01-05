//! System module for system-level utilities
//!
//! Provides sleep, timing, and other system utilities for scripts.
//!
//! Bug #1 fix: Sleep functions now use tokio::task::block_in_place to avoid
//! blocking the async runtime's worker threads. This allows other async tasks
//! to continue executing while scripts sleep.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Create the system module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("system")?;

    // Sleep/delay
    module.function("sleep", sleep_ms).build()?;
    module.function("sleep_secs", sleep_secs).build()?;

    // Time functions
    module.function("now", now_ms).build()?;
    module.function("now_secs", now_secs).build()?;
    module.function("timestamp", timestamp_iso).build()?;

    // System info
    module.function("platform", platform).build()?;
    module.function("arch", arch).build()?;
    module.function("hostname", hostname).build()?;
    module.function("username", username).build()?;
    module.function("home_dir", home_dir).build()?;

    // Process info
    module.function("pid", process_id).build()?;
    module.function("args", process_args).build()?;

    Ok(module)
}

/// Sleep for specified milliseconds
/// Bug #1 fix: Uses block_in_place to move blocking sleep off the async runtime
fn sleep_ms(ms: i64) {
    if ms > 0 && ms <= 300_000 {
        // Max 5 minutes
        // Use block_in_place to avoid blocking tokio worker threads
        // This moves the blocking operation to a dedicated blocking thread
        let _ = tokio::task::block_in_place(|| {
            std::thread::sleep(Duration::from_millis(ms as u64));
        });
    }
}

/// Sleep for specified seconds
/// Bug #1 fix: Uses block_in_place to move blocking sleep off the async runtime
fn sleep_secs(secs: i64) {
    if secs > 0 && secs <= 300 {
        // Max 5 minutes
        // Use block_in_place to avoid blocking tokio worker threads
        let _ = tokio::task::block_in_place(|| {
            std::thread::sleep(Duration::from_secs(secs as u64));
        });
    }
}

/// Get current time in milliseconds since epoch
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Get current time in seconds since epoch
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Get current timestamp in ISO 8601 format
fn timestamp_iso() -> RuneString {
    use chrono::Utc;
    let ts = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    RuneString::try_from(ts).unwrap_or_default()
}

/// Get the platform (os) name
fn platform() -> RuneString {
    RuneString::try_from(std::env::consts::OS.to_string()).unwrap_or_default()
}

/// Get the CPU architecture
fn arch() -> RuneString {
    RuneString::try_from(std::env::consts::ARCH.to_string()).unwrap_or_default()
}

/// Get the hostname
fn hostname() -> RuneString {
    match hostname::get() {
        Ok(name) => RuneString::try_from(name.to_string_lossy().to_string()).unwrap_or_default(),
        Err(_) => RuneString::new(),
    }
}

/// Get the current username
fn username() -> RuneString {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .map(|u| RuneString::try_from(u).unwrap_or_default())
        .unwrap_or_default()
}

/// Get the home directory
fn home_dir() -> RuneString {
    dirs::home_dir()
        .map(|p| RuneString::try_from(p.to_string_lossy().to_string()).unwrap_or_default())
        .unwrap_or_default()
}

/// Get the current process ID
fn process_id() -> i64 {
    std::process::id() as i64
}

/// Get command line arguments as comma-separated string
/// SECURITY: Only returns the program name, not arguments (which may contain secrets)
fn process_args() -> RuneString {
    // Only return the program name (first argument) to prevent credential leakage
    // CLI arguments like --auth="SECRET_TOKEN" should not be exposed to scripts
    let program_name = std::env::args()
        .next()
        .unwrap_or_default();
    RuneString::try_from(program_name).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let ms = now_ms();
        assert!(ms > 0);

        let secs = now_secs();
        assert!(secs > 0);
    }

    #[test]
    fn test_timestamp() {
        let ts = timestamp_iso();
        assert!(ts.contains("T"));
        assert!(ts.contains("Z"));
    }

    #[test]
    fn test_platform() {
        let p = platform();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_sleep() {
        let start = std::time::Instant::now();
        sleep_ms(50);
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 45); // Allow some tolerance
    }
}
