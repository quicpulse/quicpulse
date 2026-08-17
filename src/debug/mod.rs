//! Debug module for QuicPulse
//!
//! Provides platform detection and logging for debugging network and certificate issues.

pub mod platform;

pub use platform::PlatformInfo;
use tracing::{info, warn};

/// Log platform information and certificate diagnostics
///
/// This function detects the platform, finds certificate paths, and logs warnings
/// if no certificates are found.
pub fn log_platform_info(_url: &str) {
    let platform = platform::detect_platform();

    info!(
        os = %platform.os,
        "Platform detected"
    );

    // Log found certificate paths
    if !platform.cert_paths.is_empty() {
        for path in &platform.cert_paths {
            info!(path = %path.display(), "Certificate bundle found");
        }
    } else {
        warn!("No certificate bundles found - SSL/TLS requests may fail");
    }

    // Log certificate environment variables
    for (var, val) in &platform.cert_env_vars {
        info!(var = %var, value = %val, "Certificate environment variable");
    }
}
