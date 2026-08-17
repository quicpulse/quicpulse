//! Unit tests for Workflow Report generation (JUnit XML, JSON, TAP)

use quicpulse::pipeline::report::{generate_report, ReportConfig, ReportFormat};
use quicpulse::pipeline::runner::StepResult;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

fn create_sample_results() -> Vec<StepResult> {
    vec![
        StepResult {
            name: "Login User".to_string(),
            method: "POST".to_string(),
            url: "https://api.com/login".to_string(),
            status_code: Some(200),
            response_time: Duration::from_millis(120),
            assertions: vec![],
            extracted: Default::default(),
            error: None,
            skipped: false,
        },
        StepResult {
            name: "Fetch Profile".to_string(),
            method: "GET".to_string(),
            url: "https://api.com/profile".to_string(),
            status_code: Some(500),
            response_time: Duration::from_millis(85),
            assertions: vec![],
            extracted: Default::default(),
            error: Some("Internal server error".to_string()),
            skipped: false,
        },
    ]
}

#[test]
fn test_generate_junit_report() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("report.xml");
    let config = ReportConfig {
        output_path: file.to_str().unwrap().to_string(),
        format: ReportFormat::JUnit,
        workflow_name: "Auth Test Suite".to_string(),
        include_timing: true,
        include_response_details: true,
    };

    let results = create_sample_results();
    generate_report(&results, &config).unwrap();

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("<testsuites") || content.contains("<testsuite"));
    assert!(content.contains("Login User"));
    assert!(content.contains("Fetch Profile"));
}

#[test]
fn test_generate_json_report() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("report.json");
    let config = ReportConfig {
        output_path: file.to_str().unwrap().to_string(),
        format: ReportFormat::Json,
        workflow_name: "JSON Report".to_string(),
        include_timing: true,
        include_response_details: true,
    };

    let results = create_sample_results();
    generate_report(&results, &config).unwrap();

    let content = fs::read_to_string(&file).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        json_val.get("steps").is_some()
            || json_val.get("results").is_some()
            || json_val.is_object()
    );
}

#[test]
fn test_generate_tap_report() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("report.tap");
    let config = ReportConfig {
        output_path: file.to_str().unwrap().to_string(),
        format: ReportFormat::Tap,
        workflow_name: "TAP Report".to_string(),
        include_timing: true,
        include_response_details: true,
    };

    let results = create_sample_results();
    generate_report(&results, &config).unwrap();

    let content = fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("TAP version 13")
            || content.contains("ok 1")
            || content.contains("not ok 2")
    );
}
