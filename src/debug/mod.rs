//! Debug module for QuicPulse
//!
//! Provides platform detection and logging for debugging network issues,
//! especially on Android/Termux where certificate paths and network configuration differ.

pub mod platform;

pub use platform::PlatformInfo;
use tracing::{info, warn};

/// Log platform information and any warnings
///
/// This function detects the platform, finds certificate paths, and logs warnings
/// for common issues like missing SSL_CERT_FILE on Termux or using HTTP on Android.
pub fn log_platform_info(url: &str) {
    let platform = platform::detect_platform();

    info!(
        os = %platform.os,
        is_android = platform.is_android,
        is_termux = platform.is_termux,
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

    // Warn about HTTP on Android (cleartext blocking)
    if platform.is_android && url.starts_with("http://") {
        warn!(
            "Android 9+ blocks cleartext HTTP by default. Consider using HTTPS: {}",
            url.replace("http://", "https://")
        );
    }

    // Warn if no SSL_CERT_FILE on Termux
    if platform.is_termux && platform.cert_env_vars.is_empty() {
        warn!(
            "SSL_CERT_FILE not set in Termux. Set it with: export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem"
        );
    }
}
