//! JSON utilities
//!
//! Functions for JSON parsing and manipulation.

/// Load JSON while preserving key order
///
/// Uses serde_json with preserve_order feature.
pub fn load_json_preserve_order(s: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(s).map_err(|e| format!("JSON parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_json() {
        let result = load_json_preserve_order(r#"{"a": 1, "b": 2}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_invalid_json() {
        let result = load_json_preserve_order("not json");
        assert!(result.is_err());
    }
}
