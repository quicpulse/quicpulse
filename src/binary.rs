//! Binary data utilities
//!
//! Functions for binary data detection and formatting.

use content_inspector::{inspect, ContentType};
use humansize::{format_size, FormatSizeOptions, BINARY};

/// Check if data contains binary content
///
/// Uses statistical analysis via the content_inspector crate to detect
/// binary data. This is more robust than simple null-byte detection,
/// correctly handling text encodings like UTF-16 that may contain null bytes.
pub fn is_binary(data: &[u8]) -> bool {
    matches!(inspect(data), ContentType::BINARY)
}

/// Format byte count as human-readable size
///
/// Uses binary units (KiB, MiB, GiB, etc.)
pub fn format_bytes(bytes: u64, precision: usize) -> String {
    let options = FormatSizeOptions::from(BINARY)
        .decimal_places(precision)
        .decimal_zeroes(precision);
    format_size(bytes, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary() {
        // Binary data: random bytes
        assert!(is_binary(&[0x89, 0x50, 0x4E, 0x47])); // PNG magic bytes
        // Text data
        assert!(!is_binary(b"hello"));
        assert!(!is_binary(b"{ \"json\": true }"));
    }

    #[test]
    fn test_format_bytes() {
        // humansize crate outputs consistent format with decimal places
        assert_eq!(format_bytes(0, 2), "0.00 B");
        assert_eq!(format_bytes(100, 2), "100.00 B");
        assert_eq!(format_bytes(1024, 2), "1.00 KiB");
        assert_eq!(format_bytes(1536, 2), "1.50 KiB");
        assert_eq!(format_bytes(1048576, 2), "1.00 MiB");
    }
}
