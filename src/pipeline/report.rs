//! Report generation for workflow results
//!
//! Supports multiple output formats including JUnit XML for CI/CD integration.

use super::runner::StepResult;
use crate::errors::QuicpulseError;
use junit_report::{Duration, Report, TestCase, TestSuite};
use std::fs::File;
use std::io::Write;
use time::OffsetDateTime;

/// Report format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JUnit XML format (for CI/CD systems)
    JUnit,
    /// JSON format
    Json,
    /// TAP (Test Anything Protocol) format
    Tap,
}

/// Configuration for report generation
#[derive(Debug, Clone)]
pub struct ReportConfig {
    /// Output file path
    pub output_path: String,
    /// Report format
    pub format: ReportFormat,
    /// Workflow name (used as test suite name)
    pub workflow_name: String,
    /// Include timing information
    pub include_timing: bool,
    /// Include response details in failure messages
    pub include_response_details: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            output_path: "report.xml".to_string(),
            format: ReportFormat::JUnit,
            workflow_name: "QuicPulse Workflow".to_string(),
            include_timing: true,
            include_response_details: true,
        }
    }
}

/// Generate a report from workflow step results
pub fn generate_report(
    results: &[StepResult],
    config: &ReportConfig,
) -> Result<(), QuicpulseError> {
    match config.format {
        ReportFormat::JUnit => generate_junit_report(results, config),
        ReportFormat::Json => generate_json_report(results, config),
        ReportFormat::Tap => generate_tap_report(results, config),
    }
}

/// Generate JUnit XML report
pub fn generate_junit_report(
    results: &[StepResult],
    config: &ReportConfig,
) -> Result<(), QuicpulseError> {
    // Build test suite
    let mut suite = TestSuite::new(&config.workflow_name);
    suite.set_timestamp(OffsetDateTime::now_utc());

    for result in results {
        let test_case = build_test_case(result, config);
        suite.add_testcase(test_case);
    }

    // Build report
    let mut report = Report::new();
    report.add_testsuite(suite);

    // Write to file
    let file = File::create(&config.output_path)
        .map_err(|e| QuicpulseError::Io(e))?;

    report.write_xml(file)
        .map_err(|e| QuicpulseError::Script(format!("Failed to write JUnit XML: {}", e)))?;

    Ok(())
}

/// Build a JUnit test case from a step result
fn build_test_case(result: &StepResult, config: &ReportConfig) -> TestCase {
    // Convert std::time::Duration to time::Duration
    let duration = Duration::new(
        result.response_time.as_secs() as i64,
        result.response_time.subsec_nanos() as i32,
    );

    // Use the workflow name as classname for grouping
    let classname = sanitize_classname(&config.workflow_name);

    if result.skipped {
        // Skipped test
        let mut tc = TestCase::skipped(&result.name);
        tc.set_classname(&classname);
        tc
    } else if let Some(ref error) = result.error {
        // Error occurred (not an assertion failure, but an execution error)
        let error_message = if config.include_response_details {
            format!(
                "Step failed: {}\nMethod: {} {}\nStatus: {}",
                error,
                result.method,
                result.url,
                result.status_code.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string())
            )
        } else {
            error.clone()
        };

        let mut tc = TestCase::error(&result.name, duration, "ExecutionError", &error_message);
        tc.set_classname(&classname);
        tc
    } else {
        // Check for assertion failures
        let failed_assertions: Vec<_> = result.assertions.iter()
            .filter(|a| !a.passed)
            .collect();

        if failed_assertions.is_empty() {
            // All passed
            let mut tc = TestCase::success(&result.name, duration);
            tc.set_classname(&classname);
            tc
        } else {
            // Build failure message from all failed assertions
            let failure_messages: Vec<String> = failed_assertions.iter()
                .map(|a| format!("{}: {}", a.assertion, a.message))
                .collect();

            let failure_message = failure_messages.join("\n");

            let detailed_message = if config.include_response_details {
                format!(
                    "Assertion failures:\n{}\n\nRequest: {} {}\nStatus: {}",
                    failure_message,
                    result.method,
                    result.url,
                    result.status_code.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string())
                )
            } else {
                failure_message.clone()
            };

            let mut tc = TestCase::failure(&result.name, duration, "AssertionFailure", &detailed_message);
            tc.set_classname(&classname);
            tc
        }
    }
}

/// Sanitize a string for use as a JUnit classname
fn sanitize_classname(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

/// Generate JSON report
pub fn generate_json_report(
    results: &[StepResult],
    config: &ReportConfig,
) -> Result<(), QuicpulseError> {
    use serde_json::json;

    let report = json!({
        "name": config.workflow_name,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "total": results.len(),
            "passed": results.iter().filter(|r| r.passed()).count(),
            "failed": results.iter().filter(|r| !r.passed() && !r.skipped).count(),
            "skipped": results.iter().filter(|r| r.skipped).count(),
            "total_time_ms": results.iter().map(|r| r.response_time.as_millis()).sum::<u128>(),
        },
        "steps": results.iter().map(|r| {
            json!({
                "name": r.name,
                "method": r.method,
                "url": r.url,
                "status_code": r.status_code,
                "response_time_ms": r.response_time.as_millis(),
                "passed": r.passed(),
                "skipped": r.skipped,
                "error": r.error,
                "assertions": r.assertions.iter().map(|a| {
                    json!({
                        "assertion": a.assertion,
                        "passed": a.passed,
                        "message": a.message,
                    })
                }).collect::<Vec<_>>(),
                "extracted": r.extracted,
            })
        }).collect::<Vec<_>>(),
    });

    let json_str = serde_json::to_string_pretty(&report)
        .map_err(|e| QuicpulseError::Script(format!("Failed to serialize JSON: {}", e)))?;

    let mut file = File::create(&config.output_path)
        .map_err(|e| QuicpulseError::Io(e))?;

    file.write_all(json_str.as_bytes())
        .map_err(|e| QuicpulseError::Io(e))?;

    Ok(())
}

/// Generate TAP (Test Anything Protocol) report
pub fn generate_tap_report(
    results: &[StepResult],
    config: &ReportConfig,
) -> Result<(), QuicpulseError> {
    let mut output = String::new();

    // TAP version and plan
    output.push_str("TAP version 14\n");
    output.push_str(&format!("1..{}\n", results.len()));

    for (i, result) in results.iter().enumerate() {
        let test_num = i + 1;

        if result.skipped {
            output.push_str(&format!("ok {} - {} # SKIP\n", test_num, result.name));
        } else if result.passed() {
            output.push_str(&format!("ok {} - {} # time={}ms\n",
                test_num,
                result.name,
                result.response_time.as_millis()
            ));
        } else {
            output.push_str(&format!("not ok {} - {}\n", test_num, result.name));

            // Add diagnostic info as YAML block
            output.push_str("  ---\n");
            output.push_str(&format!("  method: {}\n", result.method));
            output.push_str(&format!("  url: {}\n", result.url));
            if let Some(status) = result.status_code {
                output.push_str(&format!("  status: {}\n", status));
            }
            if let Some(ref error) = result.error {
                output.push_str(&format!("  error: {}\n", error));
            }

            let failed_assertions: Vec<_> = result.assertions.iter()
                .filter(|a| !a.passed)
                .collect();

            if !failed_assertions.is_empty() {
                output.push_str("  failures:\n");
                for a in failed_assertions {
                    output.push_str(&format!("    - {}: {}\n", a.assertion, a.message));
                }
            }

            output.push_str("  ...\n");
        }
    }

    let mut file = File::create(&config.output_path)
        .map_err(|e| QuicpulseError::Io(e))?;

    file.write_all(output.as_bytes())
        .map_err(|e| QuicpulseError::Io(e))?;

    Ok(())
}

/// Summary of workflow execution for quick display
#[derive(Debug)]
pub struct WorkflowSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_time: std::time::Duration,
}

impl WorkflowSummary {
    pub fn from_results(results: &[StepResult]) -> Self {
        Self {
            total: results.len(),
            // Exclude skipped from passed count
            passed: results.iter().filter(|r| r.passed() && !r.skipped).count(),
            failed: results.iter().filter(|r| !r.passed() && !r.skipped).count(),
            skipped: results.iter().filter(|r| r.skipped).count(),
            total_time: results.iter().map(|r| r.response_time).sum(),
        }
    }

    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::assertions::AssertionResult;
    use std::collections::HashMap;
    use std::time::Duration as StdDuration;

    fn make_passing_result(name: &str) -> StepResult {
        StepResult {
            name: name.to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/test".to_string(),
            status_code: Some(200),
            response_time: StdDuration::from_millis(150),
            assertions: vec![
                AssertionResult::pass("status", "Status is 200"),
            ],
            extracted: HashMap::new(),
            error: None,
            skipped: false,
        }
    }

    fn make_failing_result(name: &str) -> StepResult {
        StepResult {
            name: name.to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            status_code: Some(404),
            response_time: StdDuration::from_millis(250),
            assertions: vec![
                AssertionResult::fail("status", "Expected 200, got 404"),
                AssertionResult::fail("body.id", "Field 'id' is missing"),
            ],
            extracted: HashMap::new(),
            error: None,
            skipped: false,
        }
    }

    fn make_skipped_result(name: &str) -> StepResult {
        StepResult {
            name: name.to_string(),
            method: "DELETE".to_string(),
            url: "https://api.example.com/users/1".to_string(),
            status_code: None,
            response_time: StdDuration::ZERO,
            assertions: vec![],
            extracted: HashMap::new(),
            error: None,
            skipped: true,
        }
    }

    #[test]
    fn test_workflow_summary() {
        let results = vec![
            make_passing_result("Login"),
            make_failing_result("Get Profile"),
            make_skipped_result("Logout"),
        ];

        let summary = WorkflowSummary::from_results(&results);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_junit_report_generation() {
        let results = vec![
            make_passing_result("Login"),
            make_failing_result("Get Profile"),
            make_skipped_result("Logout"),
        ];

        let config = ReportConfig {
            output_path: "/tmp/test_report.xml".to_string(),
            format: ReportFormat::JUnit,
            workflow_name: "User API Flow".to_string(),
            include_timing: true,
            include_response_details: true,
        };

        let result = generate_junit_report(&results, &config);
        assert!(result.is_ok());

        // Verify file was created and contains expected content
        let content = std::fs::read_to_string(&config.output_path).unwrap();
        assert!(content.contains("testsuites"));
        assert!(content.contains("User_API_Flow")); // classname sanitized
        assert!(content.contains("Login"));
        assert!(content.contains("Get Profile"));
        assert!(content.contains("Logout"));
        assert!(content.contains("<failure"));
        assert!(content.contains("<skipped"));

        // Cleanup
        std::fs::remove_file(&config.output_path).ok();
    }

    #[test]
    fn test_json_report_generation() {
        let results = vec![
            make_passing_result("Test Step"),
        ];

        let config = ReportConfig {
            output_path: "/tmp/test_report.json".to_string(),
            format: ReportFormat::Json,
            workflow_name: "Test Workflow".to_string(),
            include_timing: true,
            include_response_details: true,
        };

        let result = generate_json_report(&results, &config);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config.output_path).unwrap();
        assert!(content.contains("\"name\": \"Test Workflow\""));
        assert!(content.contains("\"passed\": 1"));

        std::fs::remove_file(&config.output_path).ok();
    }

    #[test]
    fn test_tap_report_generation() {
        let results = vec![
            make_passing_result("Pass Test"),
            make_failing_result("Fail Test"),
        ];

        let config = ReportConfig {
            output_path: "/tmp/test_report.tap".to_string(),
            format: ReportFormat::Tap,
            workflow_name: "TAP Test".to_string(),
            include_timing: true,
            include_response_details: true,
        };

        let result = generate_tap_report(&results, &config);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config.output_path).unwrap();
        assert!(content.contains("TAP version 14"));
        assert!(content.contains("1..2"));
        assert!(content.contains("ok 1 - Pass Test"));
        assert!(content.contains("not ok 2 - Fail Test"));

        std::fs::remove_file(&config.output_path).ok();
    }

    #[test]
    fn test_sanitize_classname() {
        assert_eq!(sanitize_classname("User API Flow"), "User_API_Flow");
        assert_eq!(sanitize_classname("test.suite"), "test.suite");
        assert_eq!(sanitize_classname("a-b-c"), "a_b_c");
    }
}
