//! Unit tests for ScriptContext, RequestData, and ResponseData state lifecycle

use quicpulse::scripting::context::{RequestData, ResponseData, ScriptContext};
use serde_json::json;

#[test]
fn test_request_data_manipulation() {
    let mut req = RequestData::new("POST", "https://api.example.com/v1/users");
    req.set_header("Authorization", "Bearer token123");
    req.set_header("Content-Type", "application/json");
    req.add_query("page", "2");
    req.add_form("role", "admin");
    req.set_body(json!({"username": "alice"}));

    assert_eq!(req.method, "POST");
    assert_eq!(req.url, "https://api.example.com/v1/users");
    assert_eq!(
        req.get_header("Authorization"),
        Some(&"Bearer token123".to_string())
    );
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.query.get("page"), Some(&"2".to_string()));
    assert_eq!(req.form.get("role"), Some(&"admin".to_string()));

    let removed = req.remove_header("Content-Type");
    assert_eq!(removed, Some("application/json".to_string()));
    assert_eq!(req.get_header("Content-Type"), None);
}

#[test]
fn test_response_data_manipulation() {
    let mut resp = ResponseData::new(200, json!({"id": 42, "user": {"name": "Bob"}}));
    resp.headers
        .insert("Content-Type".to_string(), "application/json".to_string());
    resp.elapsed_ms = 125;

    assert_eq!(resp.status, 200);
    assert!(resp.is_success());
    assert!(!resp.is_client_error());
    assert!(!resp.is_server_error());
    assert_eq!(
        resp.get_header("content-type"),
        Some(&"application/json".to_string())
    );
    assert!(resp.has_header("Content-Type"));
    assert_eq!(resp.elapsed_ms, 125);
    assert_eq!(resp.json_path(".id"), Some(json!(42)));
    assert_eq!(resp.json_path(".user.name"), Some(json!("Bob")));
}

#[test]
fn test_script_context_variables_and_extracted_state() {
    let mut ctx = ScriptContext::new();
    ctx.set_variable("user_id", json!(101));
    ctx.set_variable("auth_token", json!("xyz789"));
    ctx.set_extracted("session_key", json!("sess_abc123"));
    ctx.log("User authenticated successfully");

    assert_eq!(ctx.get_variable("user_id"), Some(&json!(101)));
    assert_eq!(ctx.get_variable("auth_token"), Some(&json!("xyz789")));
    assert_eq!(ctx.get_variable("non_existent"), None);

    assert_eq!(
        ctx.get_extracted("session_key"),
        Some(&json!("sess_abc123"))
    );
    assert_eq!(ctx.logs(), &["User authenticated successfully"]);

    ctx.clear_logs();
    assert!(ctx.logs().is_empty());

    let mut ctx2 = ScriptContext::new();
    ctx2.import_extracted(&ctx);
    assert_eq!(
        ctx2.get_extracted("session_key"),
        Some(&json!("sess_abc123"))
    );
}
