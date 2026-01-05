//! Common test utilities for quicpulse integration tests
//!
//! This module provides shared test infrastructure including:
//! - Mock HTTP server setup using wiremock
//! - CLI invocation helpers
//! - Response parsing and assertion helpers
//! - Test fixture management

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

/// HTTP 200 OK status line for assertions
pub const HTTP_OK: &str = "200 OK";

/// CRLF line ending
pub const CRLF: &str = "\r\n";

/// ANSI color escape sequence prefix
pub const COLOR: &str = "\x1b[";

/// A dummy URL that should never be resolved (for offline tests)
pub const DUMMY_URL: &str = "http://this-should.never-resolve";

/// Exit status codes matching the Rust application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success = 0,
    Error = 1,
    ErrorTimeout = 2,
    ErrorTooManyRedirects = 6,
}

impl From<i32> for ExitStatus {
    fn from(code: i32) -> Self {
        match code {
            0 => ExitStatus::Success,
            2 => ExitStatus::ErrorTimeout,
            6 => ExitStatus::ErrorTooManyRedirects,
            _ => ExitStatus::Error,
        }
    }
}

/// Result of running the HTTP CLI
#[derive(Debug)]
pub struct CliResponse {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit status code
    pub exit_status: ExitStatus,
    /// Raw exit code
    pub exit_code: i32,
    /// Parsed JSON body (if applicable)
    json_cache: Option<serde_json::Value>,
}

impl CliResponse {
    /// Parse the response body as JSON
    pub fn json(&self) -> Option<&serde_json::Value> {
        // Try to find JSON in the output
        if self.json_cache.is_some() {
            return self.json_cache.as_ref();
        }
        None
    }

    /// Check if stdout contains a substring
    pub fn contains(&self, needle: &str) -> bool {
        self.stdout.contains(needle)
    }

    /// Count occurrences of a substring in stdout
    pub fn count(&self, needle: &str) -> usize {
        self.stdout.matches(needle).count()
    }

    /// Get the response body (everything after headers)
    pub fn body(&self) -> Option<&str> {
        // Find the blank line separating headers from body
        if let Some(pos) = self.stdout.find("\r\n\r\n") {
            Some(&self.stdout[pos + 4..])
        } else if let Some(pos) = self.stdout.find("\n\n") {
            Some(&self.stdout[pos + 2..])
        } else {
            None
        }
    }
}

impl std::fmt::Display for CliResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stdout)
    }
}

impl std::ops::Deref for CliResponse {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.stdout
    }
}

/// Mock environment for testing
pub struct MockEnvironment {
    /// Temporary config directory
    pub config_dir: TempDir,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Standard input content
    pub stdin: Option<Vec<u8>>,
    /// Whether stdin is a TTY
    pub stdin_isatty: bool,
    /// Whether stdout is a TTY
    pub stdout_isatty: bool,
}

impl Default for MockEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEnvironment {
    /// Create a new mock environment
    pub fn new() -> Self {
        let config_dir = TempDir::new().expect("Failed to create temp config dir");
        Self {
            config_dir,
            env_vars: HashMap::new(),
            stdin: None,
            stdin_isatty: true,
            stdout_isatty: true,
        }
    }

    /// Set an environment variable
    pub fn set_env(&mut self, key: &str, value: &str) -> &mut Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Set stdin content
    pub fn set_stdin(&mut self, content: Vec<u8>) -> &mut Self {
        self.stdin = Some(content);
        self.stdin_isatty = false;
        self
    }

    /// Get the config directory path
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.path().to_path_buf()
    }
}

/// Run the HTTP CLI with the given arguments
///
/// # Arguments
/// * `args` - Command line arguments (excluding the program name)
///
/// # Returns
/// A `CliResponse` with stdout, stderr, and exit status
pub fn http(args: &[&str]) -> CliResponse {
    http_with_env(args, &MockEnvironment::new())
}

/// Run the HTTP CLI with the given arguments and environment
pub fn http_with_env(args: &[&str], env: &MockEnvironment) -> CliResponse {
    // Build the command - use cargo run in test mode
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_quicpulse"));
    
    // Add a timeout to prevent hanging tests (2s is plenty for mock servers)
    cmd.args(["--timeout", "2"]);
    cmd.args(args);
    
    // Set up environment
    cmd.env("QUICPULSE_CONFIG_DIR", env.config_path());
    for (key, value) in &env.env_vars {
        cmd.env(key, value);
    }
    
    // Configure stdio
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    if let Some(ref stdin_data) = env.stdin {
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn().expect("Failed to spawn command");
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(stdin_data).expect("Failed to write to stdin");
        }
        let output = child.wait_with_output().expect("Failed to wait for command");
        parse_output(output)
    } else {
        cmd.stdin(Stdio::null());
        let output = cmd.output().expect("Failed to execute command");
        parse_output(output)
    }
}

/// Run the HTTP CLI expecting an error (uses shorter timeout)
pub fn http_error(args: &[&str]) -> CliResponse {
    http_error_with_env(args, &MockEnvironment::new())
}

/// Run the HTTP CLI expecting an error with custom environment
/// Uses a shorter timeout (1s) since error conditions should fail fast
pub fn http_error_with_env(args: &[&str], env: &MockEnvironment) -> CliResponse {
    // Build the command - use cargo run in test mode
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_quicpulse"));

    // Use shorter timeout for error tests - they should fail fast
    cmd.args(["--timeout", "1"]);
    cmd.args(args);

    // Set up environment
    cmd.env("QUICPULSE_CONFIG_DIR", env.config_path());
    for (key, value) in &env.env_vars {
        cmd.env(key, value);
    }

    // Configure stdio
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(ref stdin_data) = env.stdin {
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn().expect("Failed to spawn command");
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(stdin_data).expect("Failed to write to stdin");
        }
        let output = child.wait_with_output().expect("Failed to wait for command");
        parse_output(output)
    } else {
        cmd.stdin(Stdio::null());
        let output = cmd.output().expect("Failed to execute command");
        parse_output(output)
    }
}

fn parse_output(output: Output) -> CliResponse {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(1);
    
    // Try to parse JSON from the response body
    let json_cache = extract_json(&stdout);
    
    CliResponse {
        stdout,
        stderr,
        exit_status: ExitStatus::from(exit_code),
        exit_code,
        json_cache,
    }
}

/// Try to extract JSON from the response
fn extract_json(output: &str) -> Option<serde_json::Value> {
    // If the whole output is JSON
    if output.trim().starts_with('{') || output.trim().starts_with('[') {
        if let Ok(json) = serde_json::from_str(output.trim()) {
            return Some(json);
        }
    }
    
    // Try to find JSON in the body (after headers)
    if let Some(pos) = output.find("\r\n\r\n") {
        let body = &output[pos + 4..];
        if let Ok(json) = serde_json::from_str(body.trim()) {
            return Some(json);
        }
    }
    
    if let Some(pos) = output.find("\n\n") {
        let body = &output[pos + 2..];
        if let Ok(json) = serde_json::from_str(body.trim()) {
            return Some(json);
        }
    }
    
    None
}

/// Strip ANSI color codes from a string
/// Handles all ANSI escape sequences including reset codes like \x1b[m
pub fn strip_colors(s: &str) -> String {
    // Pattern handles:
    // - \x1b[0m (reset)
    // - \x1b[m (short reset)
    // - \x1b[32m (single color)
    // - \x1b[1;32m (bold + color)
    // - \x1b[38;5;123m (256-color mode)
    // - \x1b[38;2;r;g;bm (true color)
    let re = regex::Regex::new(r"\x1b\[[\d;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

/// Create a temporary file with the given content
pub fn create_temp_file(content: &[u8]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = dir.path().join("test_file.txt");
    std::fs::write(&file_path, content).expect("Failed to write temp file");
    (dir, file_path)
}

/// Create a temporary JSON file
pub fn create_json_file(content: &serde_json::Value) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = dir.path().join("test.json");
    let json_str = serde_json::to_string_pretty(content).expect("Failed to serialize JSON");
    std::fs::write(&file_path, json_str).expect("Failed to write JSON file");
    (dir, file_path)
}

/// Test fixture paths
pub mod fixtures {
    use std::path::PathBuf;
    use once_cell::sync::Lazy;

    pub static FIXTURES_DIR: Lazy<PathBuf> = Lazy::new(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
    });

    /// Get path to a fixture file
    pub fn fixture_path(name: &str) -> PathBuf {
        FIXTURES_DIR.join(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_colors() {
        let colored = "\x1b[32mGreen\x1b[0m";
        assert_eq!(strip_colors(colored), "Green");
    }

    #[test]
    fn test_exit_status_from_i32() {
        assert_eq!(ExitStatus::from(0), ExitStatus::Success);
        assert_eq!(ExitStatus::from(1), ExitStatus::Error);
        assert_eq!(ExitStatus::from(2), ExitStatus::ErrorTimeout);
        assert_eq!(ExitStatus::from(6), ExitStatus::ErrorTooManyRedirects);
    }
}
