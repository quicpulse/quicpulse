//! String utilities
//!
//! Functions for string manipulation and version comparison.

/// Truncate a string to a maximum length, adding "..." if truncated
///
/// Handles UTF-8 character boundaries correctly.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    if max_len <= 3 {
        return "...".to_string();
    }

    // Find a safe character boundary for truncation
    let target_len = max_len - 3;
    let mut truncate_at = target_len;

    // Walk backwards to find a valid UTF-8 character boundary
    while truncate_at > 0 && !s.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }

    format!("{}...", &s[..truncate_at])
}

/// Compare two semver version strings
///
/// Returns true if v1 > v2.
/// Handles pre-release versions (e.g., 1.0.0 > 1.0.0-beta).
pub fn is_version_greater(v1: &str, v2: &str) -> bool {
    use semver::Version;

    match (Version::parse(v1), Version::parse(v2)) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hello", 3), "...");  // max_len <= 3 returns "..."
        assert_eq!(truncate_str("hi", 5), "hi");      // shorter than max_len
    }

    #[test]
    fn test_truncate_utf8() {
        // UTF-8 multi-byte characters
        let s = "héllo wörld";
        let truncated = truncate_str(s, 8);
        assert!(truncated.ends_with("..."));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn test_version_comparison() {
        assert!(is_version_greater("1.2.3", "1.2.2"));
        assert!(is_version_greater("2.0.0", "1.9.9"));
        assert!(!is_version_greater("1.0.0", "1.0.0"));
        assert!(!is_version_greater("1.0.0", "2.0.0"));
    }

    #[test]
    fn test_prerelease_versions() {
        assert!(is_version_greater("1.0.0", "1.0.0-beta"));
        assert!(!is_version_greater("1.0.0-alpha", "1.0.0"));
        assert!(is_version_greater("1.0.0-beta", "1.0.0-alpha"));
    }
}
