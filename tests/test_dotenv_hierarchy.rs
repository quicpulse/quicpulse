//! Unit tests for .env file parsing, variable expansion with defaults, and hierarchy merging

use quicpulse::devexp::dotenv::{has_variables, EnvVars};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_env_file_syntax() {
    let content = r#"
# Comments should be ignored
BASE_URL=https://api.example.com
PORT=8080
API_KEY="secret\nvalue"
SINGLE_QUOTED='literal_value'
WITH_INLINE_COMMENT=valid_part # this is a comment
"#;
    let env = EnvVars::parse(content).unwrap();

    assert_eq!(env.get("BASE_URL"), Some("https://api.example.com"));
    assert_eq!(env.get("PORT"), Some("8080"));
    assert_eq!(env.get("API_KEY"), Some("secret\nvalue"));
    assert_eq!(env.get("SINGLE_QUOTED"), Some("literal_value"));
    assert_eq!(env.get("WITH_INLINE_COMMENT"), Some("valid_part"));
}

#[test]
fn test_variable_expansion_with_defaults() {
    let mut env = EnvVars::new();
    env.set("HOST".to_string(), "127.0.0.1".to_string());
    env.set("PORT".to_string(), "9000".to_string());

    let template = "http://{{HOST}}:{{PORT}}/api/v1?timeout={{TIMEOUT:-30}}&debug={{DEBUG:-false}}";
    let expanded = env.expand(template).unwrap();

    assert_eq!(
        expanded,
        "http://127.0.0.1:9000/api/v1?timeout=30&debug=false"
    );
}

#[test]
fn test_has_variables_check() {
    assert!(has_variables("{{BASE_URL}}/users"));
    assert!(has_variables("Authorization: Bearer {{TOKEN:-default}}"));
    assert!(!has_variables("https://plain-url.com/without/vars"));
}

#[test]
fn test_env_file_load_and_merge() {
    let dir = TempDir::new().unwrap();
    let env_file1 = dir.path().join(".env.base");
    fs::write(&env_file1, "APP_ENV=local\nDEBUG=true\nSHARED=base\n").unwrap();

    let env_file2 = dir.path().join(".env.local");
    fs::write(&env_file2, "APP_ENV=staging\nSHARED=override\n").unwrap();

    let mut base_env = EnvVars::load_file(&env_file1).unwrap();
    let override_env = EnvVars::load_file(&env_file2).unwrap();

    base_env.merge(&override_env);
    assert_eq!(base_env.get("APP_ENV"), Some("staging"));
    assert_eq!(base_env.get("SHARED"), Some("override"));
    assert_eq!(base_env.get("DEBUG"), Some("true"));
}
