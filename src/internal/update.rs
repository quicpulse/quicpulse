//! Self-update functionality using the self_update crate
//!
//! This module provides a simple, standard approach to CLI self-updates
//! using GitHub releases. It replaces the custom daemon-based update checker
//! with an idiomatic Rust solution.

use std::fmt;

/// Error type for update operations
#[derive(Debug)]
pub enum UpdateError {
    /// Already running the latest version
    AlreadyUpToDate,
    /// Network or API error
    Network(String),
    /// Failed to install update
    Install(String),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::AlreadyUpToDate => write!(f, "already up to date"),
            UpdateError::Network(msg) => write!(f, "network error: {}", msg),
            UpdateError::Install(msg) => write!(f, "installation failed: {}", msg),
        }
    }
}

impl std::error::Error for UpdateError {}

/// Perform a self-update from GitHub releases
///
/// Downloads and installs the latest version from the configured GitHub repository.
/// Returns the new version string on success.
pub fn self_update() -> Result<String, UpdateError> {
    use self_update::backends::github;
    use self_update::Status;

    let current_version = env!("CARGO_PKG_VERSION");

    let status = github::Update::configure()
        .repo_owner("quicpulse")
        .repo_name("quicpulse")
        .bin_name("quicpulse")
        .current_version(current_version)
        .show_download_progress(true)
        .show_output(false)
        .no_confirm(false)
        .build()
        .map_err(|e| UpdateError::Install(e.to_string()))?
        .update()
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    match status {
        Status::UpToDate(version) => {
            // The version string is the current version when up to date
            let _ = version; // Suppress unused variable warning
            Err(UpdateError::AlreadyUpToDate)
        }
        Status::Updated(version) => {
            eprintln!("Updated to version {}", version);
            Ok(version)
        }
    }
}
