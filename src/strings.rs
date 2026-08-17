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

/// Form-style URL encode a string (uses `+` for spaces)
///
/// Unreserved characters (`A-Z a-z 0-9 - _ . ~`) pass through, a space becomes
/// `+`, and everything else is percent-encoded as uppercase UTF-8 byte pairs.
///
/// This is the single encoder for query strings and form bodies so that the
/// request the client sends, the URL shown to the user, and the exported curl
/// command all agree.
pub fn form_urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for c in s.chars() {
        match c {
            ' ' => result.push('+'),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                let encoded = c.encode_utf8(&mut buf);
                for b in encoded.as_bytes() {
                    use std::fmt::Write;
                    let _ = write!(result, "%{:02X}", b);
                }
            }
        }
    }
    result
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
        assert_eq!(truncate_str("hello", 3), "..."); // max_len <= 3 returns "..."
        assert_eq!(truncate_str("hi", 5), "hi"); // shorter than max_len
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

    #[test]
    fn test_unparseable_versions_are_never_greater() {
        // A malformed version must not be treated as an upgrade.
        assert!(!is_version_greater("not-a-version", "1.0.0"));
        assert!(!is_version_greater("1.0.0", "not-a-version"));
        assert!(!is_version_greater("", "1.0.0"));
        assert!(
            !is_version_greater("1.0", "1.0.0"),
            "semver requires 3 parts"
        );
        assert!(
            !is_version_greater("v1.2.3", "1.0.0"),
            "leading v is not semver"
        );
    }

    #[test]
    fn test_version_build_metadata_breaks_ties() {
        // The semver crate's Ord compares build metadata as a tiebreaker so
        // that Version is totally ordered, which goes beyond semver precedence.
        assert!(is_version_greater("1.0.0+build2", "1.0.0+build1"));
        assert!(!is_version_greater("1.0.0+build1", "1.0.0+build2"));
    }

    #[test]
    fn test_truncate_at_exact_max_length_is_unchanged() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_result_never_exceeds_max_len() {
        let inputs = [
            "",
            "a",
            "abcd",
            "hello world",
            "héllo wörld",
            "日本語テキスト",
        ];
        for s in inputs {
            for max_len in 0..20 {
                let out = truncate_str(s, max_len);
                if s.len() > max_len && max_len > 3 {
                    assert!(
                        out.len() <= max_len,
                        "truncate_str({s:?}, {max_len}) = {out:?} exceeds {max_len}"
                    );
                    assert!(out.ends_with("..."));
                }
            }
        }
    }

    #[test]
    fn test_truncate_tiny_limits() {
        for max_len in 0..=3 {
            assert_eq!(truncate_str("hello world", max_len), "...");
        }
    }

    #[test]
    fn test_truncate_never_splits_a_multibyte_char() {
        // "日" is 3 bytes; truncating mid-character must back off to a boundary.
        let s = "日本語";
        for max_len in 4..12 {
            let out = truncate_str(s, max_len);
            // Producing the string at all proves no panic on a bad boundary.
            assert!(out.is_char_boundary(out.len()));
            assert!(std::str::from_utf8(out.as_bytes()).is_ok());
        }
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate_str("", 0), "");
        assert_eq!(truncate_str("", 10), "");
    }

    // ---- form_urlencode ----

    #[test]
    fn test_form_urlencode_passes_unreserved_characters_through() {
        assert_eq!(
            form_urlencode("abcXYZ019-_.~"),
            "abcXYZ019-_.~",
            "unreserved characters must not be encoded"
        );
    }

    #[test]
    fn test_form_urlencode_uses_plus_for_spaces() {
        assert_eq!(form_urlencode("a b c"), "a+b+c");
    }

    #[test]
    fn test_form_urlencode_percent_encodes_reserved_characters() {
        assert_eq!(form_urlencode("a&b"), "a%26b");
        assert_eq!(form_urlencode("a=b"), "a%3Db");
        assert_eq!(form_urlencode("a?b"), "a%3Fb");
        assert_eq!(form_urlencode("a/b"), "a%2Fb");
        assert_eq!(
            form_urlencode("a+b"),
            "a%2Bb",
            "a literal + must be escaped"
        );
        assert_eq!(form_urlencode("a%b"), "a%25b");
        assert_eq!(form_urlencode("a#b"), "a%23b");
    }

    #[test]
    fn test_form_urlencode_uses_uppercase_hex() {
        // %2F not %2f - servers accept either, but stay consistent.
        let out = form_urlencode("/");
        assert_eq!(out, "%2F");
        assert!(!out.chars().any(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn test_form_urlencode_encodes_multibyte_utf8_per_byte() {
        assert_eq!(form_urlencode("é"), "%C3%A9");
        assert_eq!(form_urlencode("日"), "%E6%97%A5");
        // A 4-byte character exercises the full encode buffer.
        assert_eq!(form_urlencode("😀"), "%F0%9F%98%80");
    }

    #[test]
    fn test_form_urlencode_control_characters() {
        assert_eq!(form_urlencode("a\nb"), "a%0Ab");
        assert_eq!(form_urlencode("a\tb"), "a%09b");
    }

    #[test]
    fn test_form_urlencode_empty_string() {
        assert_eq!(form_urlencode(""), "");
    }

    #[test]
    fn test_form_urlencode_output_is_always_ascii() {
        for s in ["plain", "with space", "é日😀", "a&b=c?d/e", "\n\t\0"] {
            let out = form_urlencode(s);
            assert!(out.is_ascii(), "{s:?} produced non-ASCII output {out:?}");
        }
    }

    #[test]
    fn test_form_urlencode_round_trips_via_form_decoding() {
        // Encoding then form-decoding must return the original string.
        for original in ["hello world", "a&b=c", "é日😀", "-_.~", "a+b", "100%"] {
            let encoded = form_urlencode(original);
            let decoded: String =
                serde_urlencoded::from_str::<Vec<(String, String)>>(&format!("k={encoded}"))
                    .unwrap()
                    .into_iter()
                    .map(|(_, v)| v)
                    .collect();
            assert_eq!(decoded, original, "round trip failed for {original:?}");
        }
    }
}
