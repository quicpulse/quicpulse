//! Script type detection for multi-language scripting support
//!
//! Determines which scripting engine to use based on:
//! 1. Explicit `type` field in ScriptConfig
//! 2. File extension for external scripts
//! 3. Default to Rune for backward compatibility

use std::path::Path;

/// Supported script languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptType {
    /// Rune scripting language (default)
    #[default]
    Rune,
    /// JavaScript via QuickJS
    JavaScript,
}

impl ScriptType {
    /// Parse script type from a string (e.g., from config)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "javascript" | "js" | "ecmascript" => ScriptType::JavaScript,
            "rune" | "rn" => ScriptType::Rune,
            _ => ScriptType::Rune, // Default
        }
    }

    /// Detect script type from file extension
    pub fn from_extension(path: &str) -> Self {
        let path = Path::new(path);
        match path.extension().and_then(|e| e.to_str()) {
            Some("js") | Some("mjs") | Some("cjs") => ScriptType::JavaScript,
            Some("rn") | Some("rune") => ScriptType::Rune,
            _ => ScriptType::Rune, // Default
        }
    }

    /// Get the display name for this script type
    pub fn name(&self) -> &'static str {
        match self {
            ScriptType::Rune => "Rune",
            ScriptType::JavaScript => "JavaScript",
        }
    }

    /// Get common file extensions for this script type
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ScriptType::Rune => &["rn", "rune"],
            ScriptType::JavaScript => &["js", "mjs", "cjs"],
        }
    }
}

/// Detect script type from ScriptConfig fields
///
/// Priority:
/// 1. Explicit `type` field
/// 2. File extension (if `file` is specified)
/// 3. Default to Rune
pub fn detect_script_type(
    type_field: Option<&str>,
    file_field: Option<&str>,
) -> ScriptType {
    // Priority 1: Explicit type field
    if let Some(t) = type_field {
        return ScriptType::from_str(t);
    }

    // Priority 2: File extension
    if let Some(f) = file_field {
        return ScriptType::from_extension(f);
    }

    // Priority 3: Default to Rune
    ScriptType::Rune
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(ScriptType::from_str("javascript"), ScriptType::JavaScript);
        assert_eq!(ScriptType::from_str("js"), ScriptType::JavaScript);
        assert_eq!(ScriptType::from_str("JS"), ScriptType::JavaScript);
        assert_eq!(ScriptType::from_str("rune"), ScriptType::Rune);
        assert_eq!(ScriptType::from_str("rn"), ScriptType::Rune);
        assert_eq!(ScriptType::from_str("unknown"), ScriptType::Rune);
    }

    #[test]
    fn test_from_extension() {
        assert_eq!(ScriptType::from_extension("script.js"), ScriptType::JavaScript);
        assert_eq!(ScriptType::from_extension("script.mjs"), ScriptType::JavaScript);
        assert_eq!(ScriptType::from_extension("script.rn"), ScriptType::Rune);
        assert_eq!(ScriptType::from_extension("script.rune"), ScriptType::Rune);
        assert_eq!(ScriptType::from_extension("script.txt"), ScriptType::Rune);
        assert_eq!(ScriptType::from_extension("/path/to/file.js"), ScriptType::JavaScript);
    }

    #[test]
    fn test_detect_script_type() {
        // Explicit type takes priority
        assert_eq!(
            detect_script_type(Some("javascript"), Some("script.rn")),
            ScriptType::JavaScript
        );

        // File extension when no explicit type
        assert_eq!(
            detect_script_type(None, Some("script.js")),
            ScriptType::JavaScript
        );

        // Default to Rune
        assert_eq!(detect_script_type(None, None), ScriptType::Rune);
        assert_eq!(detect_script_type(None, Some("script.txt")), ScriptType::Rune);
    }
}
