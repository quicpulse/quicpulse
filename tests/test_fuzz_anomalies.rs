//! Unit tests for Fuzz anomaly reporting, vulnerability detection, and formatted summary

use quicpulse::fuzz::payloads::{FuzzPayload, PayloadCategory};
use quicpulse::fuzz::runner::{format_fuzz_results, FuzzResult, FuzzSummary};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn test_fuzz_result_vulnerability_classification() {
    let low_risk_anomaly = FuzzResult {
        payload: FuzzPayload {
            value: json!("'"),
            category: PayloadCategory::SqlInjection,
            description: "Single quote".to_string(),
            risk_level: 2,
        },
        field: "username".to_string(),
        status_code: Some(500),
        response_time: Duration::from_millis(45),
        error: None,
        is_anomaly: true,
        anomaly_reason: Some("HTTP 500 Internal Server Error".to_string()),
    };
    assert!(!low_risk_anomaly.is_potential_vulnerability());

    let high_risk_anomaly = FuzzResult {
        payload: FuzzPayload {
            value: json!("; cat /etc/passwd"),
            category: PayloadCategory::CommandInjection,
            description: "Command injection".to_string(),
            risk_level: 4,
        },
        field: "cmd".to_string(),
        status_code: Some(500),
        response_time: Duration::from_millis(50),
        error: None,
        is_anomaly: true,
        anomaly_reason: Some("Command executed".to_string()),
    };
    assert!(high_risk_anomaly.is_potential_vulnerability());
}

#[test]
fn test_format_fuzz_results_output() {
    let summary = FuzzSummary {
        total_requests: 20,
        successful: 18,
        client_errors: 1,
        server_errors: 1,
        timeouts: 0,
        connection_errors: 0,
        anomalies: 1,
        by_category: HashMap::new(),
        duration: Duration::from_secs(2),
    };

    let results = vec![FuzzResult {
        payload: FuzzPayload {
            value: json!("' OR '1'='1"),
            category: PayloadCategory::SqlInjection,
            description: "Basic SQLi".to_string(),
            risk_level: 3,
        },
        field: "search".to_string(),
        status_code: Some(500),
        response_time: Duration::from_millis(100),
        error: None,
        is_anomaly: true,
        anomaly_reason: Some("HTTP 500".to_string()),
    }];

    let formatted = format_fuzz_results(&results, &summary, false);
    assert!(formatted.contains("Fuzz") || formatted.contains("Total") || formatted.contains("SQL"));
    assert!(formatted.contains("anomalies") || formatted.contains("search"));
}
