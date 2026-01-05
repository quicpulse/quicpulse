//! Response assertions for CI/CD integration
//!
//! Provides assertion checks for status codes, response times, body content, and headers.

use std::time::Duration;
use reqwest::header::HeaderMap;
use serde_json::Value as JsonValue;
use crate::cli::Args;
use crate::filter;

/// Represents an assertion to check against a response
#[derive(Debug, Clone)]
pub enum Assertion {
    /// Status code assertion (e.g., "200", "2xx", "200-299")
    Status(String),
    /// Response time assertion (e.g., "<500ms", "<2s")
    Time(Duration),
    /// Body content assertion (JQ expression or literal)
    Body(String),
    /// Header assertion (name and optional value)
    Header(String, Option<String>),
}

/// Result of an assertion check
#[derive(Debug)]
pub struct AssertionResult {
    pub assertion: String,
    pub passed: bool,
    pub message: String,
}

impl AssertionResult {
    pub fn pass(assertion: &str, message: &str) -> Self {
        Self {
            assertion: assertion.to_string(),
            passed: true,
            message: message.to_string(),
        }
    }

    pub fn fail(assertion: &str, message: &str) -> Self {
        Self {
            assertion: assertion.to_string(),
            passed: false,
            message: message.to_string(),
        }
    }
}

/// Build assertions from CLI arguments
pub fn build_assertions(args: &Args) -> Vec<Assertion> {
    let mut assertions = Vec::new();

    if let Some(ref status) = args.assert_status {
        assertions.push(Assertion::Status(status.clone()));
    }

    if let Some(ref time_str) = args.assert_time {
        if let Some(duration) = parse_time_assertion(time_str) {
            assertions.push(Assertion::Time(duration));
        }
    }

    if let Some(ref body) = args.assert_body {
        assertions.push(Assertion::Body(body.clone()));
    }

    for header in &args.assert_header {
        let (name, value) = parse_header_assertion(header);
        assertions.push(Assertion::Header(name, value));
    }

    assertions
}

/// Parse time assertion string (e.g., "<500ms", "<2s")
fn parse_time_assertion(s: &str) -> Option<Duration> {
    let s = s.trim();
    let s = s.strip_prefix('<').unwrap_or(s).trim();

    // Try parsing with humantime
    humantime::parse_duration(s).ok()
}

/// Parse header assertion (e.g., "Content-Type" or "Content-Type:application/json")
fn parse_header_assertion(s: &str) -> (String, Option<String>) {
    if let Some((name, value)) = s.split_once(':') {
        (name.trim().to_string(), Some(value.trim().to_string()))
    } else {
        (s.trim().to_string(), None)
    }
}

/// Check all assertions against a response
pub fn check_assertions(
    assertions: &[Assertion],
    status_code: u16,
    response_time: Duration,
    headers: &HeaderMap,
    body: &str,
) -> Vec<AssertionResult> {
    assertions.iter().map(|assertion| {
        match assertion {
            Assertion::Status(pattern) => check_status(status_code, pattern),
            Assertion::Time(max_duration) => check_time(response_time, *max_duration),
            Assertion::Body(pattern) => check_body(body, pattern),
            Assertion::Header(name, value) => check_header(headers, name, value.as_deref()),
        }
    }).collect()
}

/// Check status code assertion
fn check_status(status_code: u16, pattern: &str) -> AssertionResult {
    let pattern = pattern.trim();
    let assertion = format!("status={}", pattern);

    // Handle exact match (e.g., "200")
    if let Ok(expected) = pattern.parse::<u16>() {
        return if status_code == expected {
            AssertionResult::pass(&assertion, &format!("Status {} matches", status_code))
        } else {
            AssertionResult::fail(&assertion, &format!("Expected {}, got {}", expected, status_code))
        };
    }

    // Handle range pattern (e.g., "200-299" or "200 - 299")
    if let Some((start, end)) = pattern.split_once('-') {
        if let (Ok(start), Ok(end)) = (start.trim().parse::<u16>(), end.trim().parse::<u16>()) {
            return if status_code >= start && status_code <= end {
                AssertionResult::pass(&assertion, &format!("Status {} in range {}-{}", status_code, start, end))
            } else {
                AssertionResult::fail(&assertion, &format!("Status {} not in range {}-{}", status_code, start, end))
            };
        }
    }

    // Handle class pattern (e.g., "2xx", "4xx")
    if pattern.len() == 3 && pattern.ends_with("xx") {
        if let Ok(class) = pattern[0..1].parse::<u16>() {
            let class_start = class * 100;
            let class_end = class_start + 99;
            return if status_code >= class_start && status_code <= class_end {
                AssertionResult::pass(&assertion, &format!("Status {} is {}xx", status_code, class))
            } else {
                AssertionResult::fail(&assertion, &format!("Status {} is not {}xx", status_code, class))
            };
        }
    }

    AssertionResult::fail(&assertion, &format!("Invalid status pattern: {}", pattern))
}

/// Check response time assertion
fn check_time(actual: Duration, max: Duration) -> AssertionResult {
    let assertion = format!("time<{:?}", max);

    if actual <= max {
        AssertionResult::pass(&assertion, &format!("Response time {:?} <= {:?}", actual, max))
    } else {
        AssertionResult::fail(&assertion, &format!("Response time {:?} > {:?}", actual, max))
    }
}

/// Check body content assertion
fn check_body(body: &str, pattern: &str) -> AssertionResult {
    let assertion = format!("body={}", pattern);

    // Try as JQ expression first (only for JSON-like patterns)
    if pattern.starts_with('.') || pattern.starts_with('[') {
        match serde_json::from_str::<JsonValue>(body) {
            Ok(json) => {
                match filter::apply_filter(&json, pattern) {
                    Ok(results) => {
                        if !results.is_empty() {
                            // Check if result is truthy
                            let is_truthy = results.iter().any(|v| match v {
                                JsonValue::Null => false,
                                JsonValue::Bool(b) => *b,
                                JsonValue::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
                                JsonValue::String(s) => !s.is_empty(),
                                JsonValue::Array(a) => !a.is_empty(),
                                JsonValue::Object(o) => !o.is_empty(),
                            });
                            if is_truthy {
                                return AssertionResult::pass(&assertion, "JQ expression matched");
                            } else {
                                return AssertionResult::fail(&assertion, "JQ expression returned falsy value");
                            }
                        }
                        // JQ expression returned no results
                        return AssertionResult::fail(&assertion, "JQ expression found no matches");
                    }
                    Err(e) => {
                        // JQ filter error
                        return AssertionResult::fail(&assertion, &format!("JQ filter error: {}", e));
                    }
                }
            }
            Err(_) => {
                // Body is not JSON, can't use JQ expression
                return AssertionResult::fail(&assertion, "Response is not JSON, cannot use JQ expression");
            }
        }
    }

    // Try as key:value pattern (e.g., "success:true")
    if let Some((key, expected_value)) = pattern.split_once(':') {
        // Try JSON first
        if let Ok(json) = serde_json::from_str::<JsonValue>(body) {
            let jq_expr = format!(".{}", key);
            if let Ok(results) = filter::apply_filter(&json, &jq_expr) {
                if !results.is_empty() {
                    let actual_str = results[0].to_string();
                    let actual_str = actual_str.trim_matches('"');
                    if actual_str == expected_value {
                        return AssertionResult::pass(&assertion, &format!("{} = {}", key, expected_value));
                    } else {
                        return AssertionResult::fail(&assertion, &format!("{} = {} (expected {})", key, actual_str, expected_value));
                    }
                }
            }
        }
        // For non-JSON, fall through to literal match
    }

    // Fall back to literal substring match (works for any content type)
    if body.contains(pattern) {
        AssertionResult::pass(&assertion, "Body contains pattern")
    } else {
        AssertionResult::fail(&assertion, "Body does not contain pattern")
    }
}

/// Check header assertion
fn check_header(headers: &HeaderMap, name: &str, expected_value: Option<&str>) -> AssertionResult {
    let assertion = if let Some(val) = expected_value {
        format!("header={}:{}", name, val)
    } else {
        format!("header={}", name)
    };

    // Find header (case-insensitive)
    let header_value = headers.iter()
        .find(|(k, _)| k.as_str().eq_ignore_ascii_case(name))
        .and_then(|(_, v)| v.to_str().ok());

    match (header_value, expected_value) {
        (Some(actual), Some(expected)) => {
            if actual.contains(expected) {
                AssertionResult::pass(&assertion, &format!("{}: {} contains {}", name, actual, expected))
            } else {
                AssertionResult::fail(&assertion, &format!("{}: {} does not match {}", name, actual, expected))
            }
        }
        (Some(actual), None) => {
            AssertionResult::pass(&assertion, &format!("Header {} present: {}", name, actual))
        }
        (None, _) => {
            AssertionResult::fail(&assertion, &format!("Header {} not found", name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_exact() {
        let result = check_status(200, "200");
        assert!(result.passed);

        let result = check_status(404, "200");
        assert!(!result.passed);
    }

    #[test]
    fn test_status_range() {
        let result = check_status(201, "200-299");
        assert!(result.passed);

        let result = check_status(404, "200-299");
        assert!(!result.passed);
    }

    #[test]
    fn test_status_class() {
        let result = check_status(201, "2xx");
        assert!(result.passed);

        let result = check_status(404, "2xx");
        assert!(!result.passed);

        let result = check_status(503, "5xx");
        assert!(result.passed);
    }

    #[test]
    fn test_time_assertion() {
        let result = check_time(Duration::from_millis(100), Duration::from_millis(500));
        assert!(result.passed);

        let result = check_time(Duration::from_millis(600), Duration::from_millis(500));
        assert!(!result.passed);
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time_assertion("<500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_time_assertion("< 2s"), Some(Duration::from_secs(2)));
        assert_eq!(parse_time_assertion("1s"), Some(Duration::from_secs(1)));
    }

    #[test]
    fn test_body_literal() {
        let result = check_body(r#"{"success": true}"#, "success");
        assert!(result.passed);

        let result = check_body(r#"{"error": "not found"}"#, "success");
        assert!(!result.passed);
    }

    #[test]
    fn test_body_key_value() {
        let result = check_body(r#"{"success": true}"#, "success:true");
        assert!(result.passed);

        let result = check_body(r#"{"success": false}"#, "success:true");
        assert!(!result.passed);
    }
}
