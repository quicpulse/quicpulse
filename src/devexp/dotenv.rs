//! .env file loading and variable expansion
//!
//! Supports loading environment variables from .env files and expanding
//! {{variable}} syntax in request arguments.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::errors::QuicpulseError;

/// SIMD-optimized regex for variable expansion: {{VAR_NAME}} or {{VAR_NAME:-default}}
static EXPAND_VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{([A-Za-z_][A-Za-z0-9_]*)(?::-([^}]*))?\}\}").expect("Invalid expand regex")
});

/// SIMD-optimized regex for detecting variables
static HAS_VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{[A-Za-z_][A-Za-z0-9_]*(?::-[^}]*)?\}\}").expect("Invalid has_var regex")
});

/// Environment variable store
#[derive(Debug, Clone, Default)]
pub struct EnvVars {
    vars: HashMap<String, String>,
}

impl EnvVars {
    /// Create a new empty EnvVars
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Load from system environment variables
    pub fn from_env() -> Self {
        let vars: HashMap<String, String> = std::env::vars().collect();
        Self { vars }
    }

    /// Load from a .env file
    pub fn load_file(path: &Path) -> Result<Self, QuicpulseError> {
        let content = fs::read_to_string(path).map_err(|e| {
            QuicpulseError::Config(format!("Failed to read .env file: {}", e))
        })?;

        Self::parse(&content)
    }

    /// Try to load .env from current directory (returns empty if not found)
    pub fn try_load_default() -> Self {
        let env_path = Path::new(".env");
        if env_path.exists() {
            Self::load_file(env_path).unwrap_or_default()
        } else {
            Self::new()
        }
    }

    /// Parse .env file content
    pub fn parse(content: &str) -> Result<Self, QuicpulseError> {
        let mut vars = HashMap::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=value or KEY="value" or KEY='value'
            if let Some((key, value)) = parse_env_line(line) {
                vars.insert(key, value);
            } else {
                return Err(QuicpulseError::Config(format!(
                    "Invalid .env syntax at line {}: {}",
                    line_num + 1,
                    line
                )));
            }
        }

        Ok(Self { vars })
    }

    /// Merge with another EnvVars (other takes precedence)
    pub fn merge(&mut self, other: &EnvVars) {
        for (key, value) in &other.vars {
            self.vars.insert(key.clone(), value.clone());
        }
    }

    /// Merge with system environment (system takes precedence over file)
    pub fn merge_with_system(&mut self) {
        for (key, value) in std::env::vars() {
            self.vars.insert(key, value);
        }
    }

    /// Get a variable value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    /// Set a variable
    pub fn set(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    /// Check if a variable exists
    pub fn contains(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Get all variables
    pub fn all(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// Expand {{variable}} syntax in a string
    pub fn expand(&self, input: &str) -> Result<String, QuicpulseError> {
        expand_variables(input, &self.vars)
    }
}

/// Parse a single .env line into key-value pair
fn parse_env_line(line: &str) -> Option<(String, String)> {
    // Find the = sign
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim();

    if key.is_empty() {
        return None;
    }

    let value_part = line[eq_pos + 1..].trim();

    // Handle quoted values
    let value = if value_part.starts_with('"') && value_part.ends_with('"') && value_part.len() >= 2 {
        // Double-quoted: process escape sequences
        unescape_double_quoted(&value_part[1..value_part.len() - 1])
    } else if value_part.starts_with('\'') && value_part.ends_with('\'') && value_part.len() >= 2 {
        // Single-quoted: literal value
        value_part[1..value_part.len() - 1].to_string()
    } else {
        // Unquoted: trim and take as-is, stop at # (comment)
        let value = if let Some(comment_pos) = value_part.find('#') {
            value_part[..comment_pos].trim()
        } else {
            value_part
        };
        value.to_string()
    };

    Some((key.to_string(), value))
}

/// Unescape double-quoted string
fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('$') => result.push('$'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Expand {{variable}} syntax in a string
/// Uses SIMD-optimized cached regex for performance
pub fn expand_variables(input: &str, vars: &HashMap<String, String>) -> Result<String, QuicpulseError> {
    let mut result = input.to_string();
    let mut missing: Vec<String> = Vec::new();

    // Find all matches and collect replacements using cached regex
    let replacements: Vec<(String, String)> = EXPAND_VAR_RE.captures_iter(input)
        .map(|cap| {
            let full_match = cap.get(0).unwrap().as_str().to_string();
            let var_name = cap.get(1).unwrap().as_str();
            let default = cap.get(2).map(|m| m.as_str());

            let value = if let Some(val) = vars.get(var_name) {
                val.clone()
            } else if let Ok(val) = std::env::var(var_name) {
                val
            } else if let Some(def) = default {
                def.to_string()
            } else {
                missing.push(var_name.to_string());
                full_match.clone() // Keep original if not found
            };

            (full_match, value)
        })
        .collect();

    // Apply replacements
    for (pattern, value) in replacements {
        result = result.replacen(&pattern, &value, 1);
    }

    // If there are missing variables without defaults, return error
    if !missing.is_empty() {
        return Err(QuicpulseError::Config(format!(
            "Undefined environment variables: {}",
            missing.join(", ")
        )));
    }

    Ok(result)
}

/// Check if a string contains {{variable}} syntax
/// Uses SIMD-optimized cached regex for performance
pub fn has_variables(s: &str) -> bool {
    HAS_VAR_RE.is_match(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let content = "KEY=value";
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some("value"));
    }

    #[test]
    fn test_parse_quoted() {
        let content = r#"KEY="hello world""#;
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some("hello world"));
    }

    #[test]
    fn test_parse_single_quoted() {
        let content = "KEY='hello world'";
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some("hello world"));
    }

    #[test]
    fn test_parse_escape_sequences() {
        let content = r#"KEY="line1\nline2""#;
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some("line1\nline2"));
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r#"
# This is a comment
KEY=value # inline comment
OTHER=test
"#;
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some("value"));
        assert_eq!(env.get("OTHER"), Some("test"));
    }

    #[test]
    fn test_parse_empty_value() {
        let content = "KEY=";
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("KEY"), Some(""));
    }

    #[test]
    fn test_expand_simple() {
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "world".to_string());

        let result = expand_variables("Hello {{NAME}}", &vars).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_expand_multiple() {
        let mut vars = HashMap::new();
        vars.insert("FIRST".to_string(), "Hello".to_string());
        vars.insert("SECOND".to_string(), "World".to_string());

        let result = expand_variables("{{FIRST}} {{SECOND}}!", &vars).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_expand_with_default() {
        let vars = HashMap::new();
        let result = expand_variables("Hello {{NAME:-stranger}}", &vars).unwrap();
        assert_eq!(result, "Hello stranger");
    }

    #[test]
    fn test_expand_default_not_used() {
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "friend".to_string());

        let result = expand_variables("Hello {{NAME:-stranger}}", &vars).unwrap();
        assert_eq!(result, "Hello friend");
    }

    #[test]
    fn test_expand_missing_error() {
        let vars = HashMap::new();
        let result = expand_variables("Hello {{MISSING}}", &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_variables() {
        assert!(has_variables("{{VAR}}"));
        assert!(has_variables("prefix {{VAR}} suffix"));
        assert!(has_variables("{{VAR:-default}}"));
        assert!(!has_variables("no variables here"));
        assert!(!has_variables("{single_brace}"));
    }

    #[test]
    fn test_parse_multiline() {
        let content = r#"
API_KEY=secret123
BASE_URL=https://api.example.com
DEBUG=true
"#;
        let env = EnvVars::parse(content).unwrap();
        assert_eq!(env.get("API_KEY"), Some("secret123"));
        assert_eq!(env.get("BASE_URL"), Some("https://api.example.com"));
        assert_eq!(env.get("DEBUG"), Some("true"));
    }
}
