//! Exit status codes for the CLI
//!
//! QuicPulse follows standard Unix exit code conventions:
//! - 0: Success
//! - 1: Any error (network, HTTP errors with --check-status, timeouts, etc.)
//! - 130: User interrupted (Ctrl+C, standard SIGINT exit code)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminants_follow_unix_conventions() {
        assert_eq!(ExitStatus::Success as u8, 0);
        assert_eq!(ExitStatus::Error as u8, 1);
        assert_eq!(ExitStatus::Interrupted as u8, 130);
        assert_eq!(EXIT_ASSERTION_FAILED, 10);
    }

    #[test]
    fn test_from_http_status_without_check_status_always_succeeds() {
        // Without --check-status, HTTP errors are not application errors.
        for code in [200, 301, 404, 418, 500, 503] {
            assert_eq!(
                ExitStatus::from_http_status(code, false),
                ExitStatus::Success,
                "status {code}"
            );
        }
    }

    #[test]
    fn test_from_http_status_with_check_status_maps_2xx_to_success() {
        for code in [200, 201, 204, 250, 299] {
            assert_eq!(
                ExitStatus::from_http_status(code, true),
                ExitStatus::Success,
                "status {code}"
            );
        }
    }

    #[test]
    fn test_from_http_status_with_check_status_maps_non_2xx_to_error() {
        for code in [100, 199, 300, 301, 400, 404, 500, 503] {
            assert_eq!(
                ExitStatus::from_http_status(code, true),
                ExitStatus::Error,
                "status {code}"
            );
        }
    }

    #[test]
    fn test_from_http_status_boundaries() {
        // The success window is exactly 200..=299.
        assert_eq!(ExitStatus::from_http_status(199, true), ExitStatus::Error);
        assert_eq!(ExitStatus::from_http_status(200, true), ExitStatus::Success);
        assert_eq!(ExitStatus::from_http_status(299, true), ExitStatus::Success);
        assert_eq!(ExitStatus::from_http_status(300, true), ExitStatus::Error);
    }

    #[test]
    fn test_from_code_recognizes_success_and_interrupt() {
        assert_eq!(ExitStatus::from_code(0), ExitStatus::Success);
        assert_eq!(ExitStatus::from_code(130), ExitStatus::Interrupted);
    }

    #[test]
    fn test_from_code_treats_everything_else_as_error() {
        for code in [1, 2, 10, 42, 129, 131, 255, -1] {
            assert_eq!(
                ExitStatus::from_code(code),
                ExitStatus::Error,
                "code {code}"
            );
        }
    }

    #[test]
    fn test_from_code_round_trips_its_own_discriminants() {
        for status in [
            ExitStatus::Success,
            ExitStatus::Error,
            ExitStatus::Interrupted,
        ] {
            assert_eq!(ExitStatus::from_code(status as i32), status);
        }
    }

    #[test]
    fn test_converts_into_process_exit_code() {
        // ExitCode has no PartialEq, so compare its Debug rendering against
        // an ExitCode built from the same raw byte.
        for status in [
            ExitStatus::Success,
            ExitStatus::Error,
            ExitStatus::Interrupted,
        ] {
            let converted = ExitCode::from(status);
            assert_eq!(
                format!("{:?}", converted),
                format!("{:?}", ExitCode::from(status as u8)),
                "{status:?}"
            );
        }
    }

    #[test]
    fn test_termination_report_matches_the_conversion() {
        for status in [
            ExitStatus::Success,
            ExitStatus::Error,
            ExitStatus::Interrupted,
        ] {
            assert_eq!(
                format!("{:?}", status.report()),
                format!("{:?}", ExitCode::from(status)),
                "{status:?}"
            );
        }
    }
}
