//! Platform detection for debugging purposes
//!
//! This module helps detect the platform (OS, Android, Termux) and find certificate paths.
//! This is particularly useful for diagnosing issues on Android/Termux where certificates
//! are in non-standard locations.

use std::env;
use std::path::PathBuf;

/// Platform information for debugging
pub struct PlatformInfo {
    pub os: String,
    pub is_android: bool,
    pub is_termux: bool,
    pub cert_paths: Vec<PathBuf>,
    pub cert_env_vars: Vec<(String, String)>,
}

/// Detect the current platform and certificate configuration
pub fn detect_platform() -> PlatformInfo {
    let os = std::env::consts::OS.to_string();
    let is_termux = is_termux_environment();
    let is_android = is_termux || is_android_system();

    PlatformInfo {
        os,
        is_android,
        is_termux,
        cert_paths: find_cert_paths(),
        cert_env_vars: get_cert_env_vars(),
    }
}

/// Check if running in Termux environment
fn is_termux_environment() -> bool {
    if let Ok(prefix) = env::var("PREFIX") {
        return prefix.contains("com.termux");
    }
    false
}

/// Check if running on Android system (even outside Termux)
fn is_android_system() -> bool {
    std::path::Path::new("/system/build.prop").exists()
}

/// Find all available certificate bundle files
fn find_cert_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Check SSL_CERT_FILE environment variable
    if let Ok(path) = env::var("SSL_CERT_FILE") {
        let p = PathBuf::from(&path);
        if p.exists() {
            paths.push(p);
        }
    }

    // Check Termux certificate path
    if let Ok(prefix) = env::var("PREFIX") {
        let termux_cert = PathBuf::from(format!("{}/etc/tls/cert.pem", prefix));
        if termux_cert.exists() {
            paths.push(termux_cert);
        }
    }

    // Standard Linux certificate paths
    for path in &["/etc/ssl/certs/ca-certificates.crt", "/etc/ssl/cert.pem"] {
        let p = PathBuf::from(path);
        if p.exists() {
            paths.push(p);
        }
    }

    paths
}

/// Get certificate-related environment variables
fn get_cert_env_vars() -> Vec<(String, String)> {
    let mut vars = Vec::new();
    for var in &["SSL_CERT_FILE", "SSL_CERT_DIR"] {
        if let Ok(value) = env::var(var) {
            vars.push((var.to_string(), value));
        }
    }
    vars
}
