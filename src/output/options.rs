//! Output options
//!
//! Provides type-safe output configuration using bitflags for component
//! selection ("hHbB" style) and PrettyOption for formatting control.

use bitflags::bitflags;
use clap::ValueEnum;

/// Pretty printing options
///
/// Controls how output is formatted and colorized.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum PrettyOption {
    /// Apply both colors and formatting
    #[default]
    All,
    /// Apply colors only
    Colors,
    /// Apply formatting only
    Format,
    /// Disable pretty printing
    None,
}

impl PrettyOption {
    /// Check if colors should be applied
    pub fn uses_colors(&self) -> bool {
        matches!(self, Self::All | Self::Colors)
    }

    /// Check if formatting should be applied
    pub fn uses_formatting(&self) -> bool {
        matches!(self, Self::All | Self::Format)
    }
}

bitflags! {
    /// Output component flags
    ///
    /// Controls which parts of the HTTP exchange are displayed.
    /// Uses bitflags for efficient storage and type-safe operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct OutputFlags: u8 {
        /// Show request headers (H)
        const REQUEST_HEADERS  = 0b0000_0001;
        /// Show request body (B)
        const REQUEST_BODY     = 0b0000_0010;
        /// Show response headers (h)
        const RESPONSE_HEADERS = 0b0000_0100;
        /// Show response body (b)
        const RESPONSE_BODY    = 0b0000_1000;
        /// Show metadata (m) - timing, sizes, etc.
        const METADATA         = 0b0001_0000;

        // Common presets
        /// Full verbose output: request + response headers and bodies
        const VERBOSE = Self::REQUEST_HEADERS.bits()
                      | Self::REQUEST_BODY.bits()
                      | Self::RESPONSE_HEADERS.bits()
                      | Self::RESPONSE_BODY.bits();

        /// Default output: response headers + body
        const DEFAULT = Self::RESPONSE_HEADERS.bits()
                      | Self::RESPONSE_BODY.bits();

        /// Redirected output: body only
        const REDIRECTED = Self::RESPONSE_BODY.bits();

        /// Offline mode: request only
        const OFFLINE = Self::REQUEST_HEADERS.bits()
                      | Self::REQUEST_BODY.bits();

        /// Request only
        const REQUEST = Self::REQUEST_HEADERS.bits()
                      | Self::REQUEST_BODY.bits();

        /// Response only
        const RESPONSE = Self::RESPONSE_HEADERS.bits()
                       | Self::RESPONSE_BODY.bits();

        /// Headers only (request + response)
        const HEADERS = Self::REQUEST_HEADERS.bits()
                      | Self::RESPONSE_HEADERS.bits();

        /// Bodies only (request + response)
        const BODIES = Self::REQUEST_BODY.bits()
                     | Self::RESPONSE_BODY.bits();
    }
}

impl OutputFlags {
    /// Parse from CLI print string (e.g., "hHbB", "h", "b")
    ///
    /// Characters:
    /// - H: request headers
    /// - B: request body
    /// - h: response headers
    /// - b: response body
    /// - m: metadata
    pub fn from_print_str(s: &str) -> Self {
        let mut flags = Self::empty();
        for c in s.chars() {
            match c {
                'H' => flags |= Self::REQUEST_HEADERS,
                'B' => flags |= Self::REQUEST_BODY,
                'h' => flags |= Self::RESPONSE_HEADERS,
                'b' => flags |= Self::RESPONSE_BODY,
                'm' => flags |= Self::METADATA,
                _ => {} // Ignore unknown chars
            }
        }
        flags
    }

    /// Convert to CLI print string format
    pub fn to_print_str(&self) -> String {
        let mut s = String::with_capacity(5);
        if self.contains(Self::REQUEST_HEADERS) {
            s.push('H');
        }
        if self.contains(Self::REQUEST_BODY) {
            s.push('B');
        }
        if self.contains(Self::RESPONSE_HEADERS) {
            s.push('h');
        }
        if self.contains(Self::RESPONSE_BODY) {
            s.push('b');
        }
        if self.contains(Self::METADATA) {
            s.push('m');
        }
        s
    }

    /// Check if request headers should be shown
    pub fn show_request_headers(&self) -> bool {
        self.contains(Self::REQUEST_HEADERS)
    }

    /// Check if request body should be shown
    pub fn show_request_body(&self) -> bool {
        self.contains(Self::REQUEST_BODY)
    }

    /// Check if response headers should be shown
    pub fn show_response_headers(&self) -> bool {
        self.contains(Self::RESPONSE_HEADERS)
    }

    /// Check if response body should be shown
    pub fn show_response_body(&self) -> bool {
        self.contains(Self::RESPONSE_BODY)
    }

    /// Check if metadata should be shown
    pub fn show_metadata(&self) -> bool {
        self.contains(Self::METADATA)
    }

    /// Check if any request components are enabled
    pub fn has_request(&self) -> bool {
        self.intersects(Self::REQUEST_HEADERS | Self::REQUEST_BODY)
    }

    /// Check if any response components are enabled
    pub fn has_response(&self) -> bool {
        self.intersects(Self::RESPONSE_HEADERS | Self::RESPONSE_BODY)
    }
}

impl Default for OutputFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for OutputFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_print_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_print_str() {
        assert_eq!(
            OutputFlags::from_print_str("hb"),
            OutputFlags::RESPONSE_HEADERS | OutputFlags::RESPONSE_BODY
        );
        assert_eq!(
            OutputFlags::from_print_str("HBhb"),
            OutputFlags::VERBOSE
        );
        assert_eq!(OutputFlags::from_print_str("b"), OutputFlags::RESPONSE_BODY);
        assert_eq!(OutputFlags::from_print_str(""), OutputFlags::empty());
    }

    #[test]
    fn test_to_print_str() {
        assert_eq!(OutputFlags::DEFAULT.to_print_str(), "hb");
        assert_eq!(OutputFlags::VERBOSE.to_print_str(), "HBhb");
        assert_eq!(OutputFlags::REDIRECTED.to_print_str(), "b");
    }

    #[test]
    fn test_roundtrip() {
        let flags = OutputFlags::REQUEST_HEADERS | OutputFlags::RESPONSE_BODY | OutputFlags::METADATA;
        let s = flags.to_print_str();
        assert_eq!(OutputFlags::from_print_str(&s), flags);
    }

    #[test]
    fn test_presets() {
        assert!(OutputFlags::DEFAULT.show_response_headers());
        assert!(OutputFlags::DEFAULT.show_response_body());
        assert!(!OutputFlags::DEFAULT.show_request_headers());

        assert!(OutputFlags::OFFLINE.show_request_headers());
        assert!(OutputFlags::OFFLINE.show_request_body());
        assert!(!OutputFlags::OFFLINE.show_response_headers());
    }

    #[test]
    fn test_has_request_response() {
        assert!(OutputFlags::VERBOSE.has_request());
        assert!(OutputFlags::VERBOSE.has_response());
        assert!(!OutputFlags::REDIRECTED.has_request());
        assert!(OutputFlags::REDIRECTED.has_response());
    }
}
