//! Platform detection for debugging purposes
//!
//! This module helps detect the platform (OS) and find certificate paths.

use std::env;
use std::path::PathBuf;

/// Platform information for debugging
pub struct PlatformInfo {
    pub os: String,
    pub cert_paths: Vec<PathBuf>,
    pub cert_env_vars: Vec<(String, String)>,
}

/// Detect the current platform and certificate configuration
pub fn detect_platform() -> PlatformInfo {
    let os = std::env::consts::OS.to_string();

    PlatformInfo {
        os,
        cert_paths: find_cert_paths(),
        cert_env_vars: get_cert_env_vars(),
    }
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

    // Standard Linux / Unix certificate paths
    for path in &[
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/ssl/cert.pem",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/ca-bundle.pem",
    ] {
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
