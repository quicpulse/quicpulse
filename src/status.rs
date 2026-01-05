//! Exit status codes for the CLI
//!
//! QuicPulse follows standard Unix exit code conventions:
//! - 0: Success
//! - 1: Any error (network, HTTP errors with --check-status, timeouts, etc.)
//! - 130: User interrupted (Ctrl+C, standard SIGINT exit code)
//!
//! This is a clean room design that follows standard Unix practices rather than
//! using application-specific exit codes for different error types. Users who
//! need to distinguish between HTTP 4xx vs 5xx errors can use --check-status
//! combined with shell conditionals and response inspection.

use std::process::{ExitCode, Termination};

/// Exit status codes following standard Unix conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitStatus {
    /// Successful execution (HTTP 2xx or no --check-status)
    Success = 0,
    /// Any error (HTTP 3xx/4xx/5xx with --check-status, timeouts, connection errors)
    Error = 1,
    /// User interrupted (Ctrl+C) - standard SIGINT code
    Interrupted = 130,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        ExitCode::from(status as u8)
    }
}

impl Termination for ExitStatus {
    fn report(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

/// Exit code for assertion failures (used in workflow/pipeline mode)
pub const EXIT_ASSERTION_FAILED: i32 = 10;

impl ExitStatus {
    /// Create an exit status from an HTTP status code with --check-status flag
    ///
    /// When check_status is true:
    /// - 2xx responses return Success
    /// - 3xx/4xx/5xx responses return Error
    ///
    /// When check_status is false, always returns Success (HTTP errors are not
    /// considered application errors unless explicitly checked).
    pub fn from_http_status(status_code: u16, check_status: bool) -> Self {
        if !check_status || (200..300).contains(&status_code) {
            ExitStatus::Success
        } else {
            ExitStatus::Error
        }
    }

    /// Create an exit status from a raw exit code
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => ExitStatus::Success,
            130 => ExitStatus::Interrupted,
            _ => ExitStatus::Error,
        }
    }
}
