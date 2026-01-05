//! Filesystem utilities
//!
//! Functions for filename handling and sanitization.

use content_disposition::parse_content_disposition;
use sanitize_filename::Options as SanitizeOptions;

/// Extract filename from Content-Disposition header
///
/// Handles both RFC 5987 encoded (filename*=) and regular (filename=) formats.
pub fn get_filename_from_content_disposition(header: &str) -> Option<String> {
    let parsed = parse_content_disposition(header);
    parsed.filename_full()
}

/// Sanitize a filename for safe filesystem usage
///
/// Uses the sanitize-filename crate for cross-platform safe filenames.
/// Replaces invalid characters and Windows reserved names.
pub fn sanitize_filename(name: &str) -> String {
    // Use windows mode to also handle reserved names like CON, NUL, etc.
    sanitize_filename::sanitize_with_options(name, SanitizeOptions {
        replacement: "_",
        windows: true,
        truncate: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file:name.txt"), "file_name.txt");
        assert_eq!(sanitize_filename("path/to/file"), "path_to_file");
        assert_eq!(sanitize_filename("safe_file.txt"), "safe_file.txt");
    }

    #[test]
    fn test_sanitize_reserved_names() {
        // Windows reserved names are replaced entirely for safety
        assert_eq!(sanitize_filename("CON.txt"), "_");
        assert_eq!(sanitize_filename("NUL"), "_");
    }

    #[test]
    fn test_content_disposition() {
        let header = "attachment; filename=\"report.pdf\"";
        assert_eq!(get_filename_from_content_disposition(header), Some("report.pdf".to_string()));
    }
}
