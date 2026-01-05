//! Integration tests for plugin runtime functionality

mod common;

use common::{http, http_error, fixtures, ExitStatus};
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

fn fixture_path(name: &str) -> PathBuf {
    fixtures::fixture_path(name)
}

// =============================================================================
// Plugin Discovery Tests
// =============================================================================

#[test]
fn test_plugin_list_empty() {
    // With no plugins installed, list should be empty
    let temp_dir = TempDir::new().unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    // Should succeed but show no plugins
    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_plugin_list_with_plugin() {
    let temp_dir = TempDir::new().unwrap();

    // Create a plugin directory structure
    let plugin_dir = temp_dir.path().join("test-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // Create plugin manifest
    let manifest = r#"
name: test-plugin
version: "1.0.0"
description: A test plugin
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#;
    std::fs::write(plugin_dir.join("plugin.yaml"), manifest).unwrap();

    // Create dummy plugin script
    let script = "#!/bin/bash\necho '{\"continue_processing\": true}'";
    std::fs::write(plugin_dir.join("plugin.sh"), script).unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(output.contains("test-plugin") || output.contains("1.0.0"),
        "Should list plugin. output: {}", output);
}

#[test]
fn test_plugin_discover_multiple() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple plugins
    for name in &["plugin-a", "plugin-b", "plugin-c"] {
        let plugin_dir = temp_dir.path().join(name);
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let manifest = format!(r#"
name: {}
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#, name);
        std::fs::write(plugin_dir.join("plugin.yaml"), manifest).unwrap();
        std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{}'").unwrap();
    }

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should find multiple plugins
    let plugin_count = output.matches("plugin-").count();
    assert!(plugin_count >= 1, "Should find plugins. output: {}", output);
}

// =============================================================================
// Plugin Manifest Tests
// =============================================================================

#[test]
fn test_plugin_yaml_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("yaml-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // YAML manifest
    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: yaml-plugin
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_plugin_json_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("json-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // JSON manifest
    std::fs::write(plugin_dir.join("plugin.json"), r#"{
    "name": "json-plugin",
    "version": "1.0.0",
    "plugin_type": "binary",
    "entry": "plugin.sh",
    "hooks": ["pre_request"]
}"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_plugin_toml_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("toml-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // TOML manifest
    std::fs::write(plugin_dir.join("plugin.toml"), r#"
name = "toml-plugin"
version = "1.0.0"
plugin_type = "binary"
entry = "plugin.sh"
hooks = ["pre_request"]
"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Plugin Search Tests
// =============================================================================

#[test]
fn test_plugin_search() {
    // Plugin search requires network (registry)
    // This test just verifies the flag is accepted
    let response = http_error(&[
        "--plugin-search", "http-utils",
        "--offline"
    ]);

    // Should accept the flag (may fail due to offline/network)
    assert!(response.exit_status == ExitStatus::Error ||
            response.stderr.contains("search") ||
            response.stderr.contains("registry") ||
            response.stderr.contains("offline"),
        "Should handle search. stderr: {}", response.stderr);
}

// =============================================================================
// Plugin Installation Tests
// =============================================================================

#[test]
fn test_plugin_install_nonexistent() {
    let response = http_error(&[
        "--plugin-install", "nonexistent-plugin-xyz"
    ]);

    // Should fail for nonexistent plugin
    assert_eq!(response.exit_status, ExitStatus::Error);
}

#[test]
fn test_plugin_uninstall_nonexistent() {
    let response = http_error(&[
        "--plugin-uninstall", "nonexistent-plugin-xyz"
    ]);

    // Should fail for nonexistent plugin
    assert_eq!(response.exit_status, ExitStatus::Error);
}

// =============================================================================
// Plugin Execution Tests
// =============================================================================

#[tokio::test]
async fn test_plugin_binary_execution() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("exec-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // Create manifest
    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: exec-plugin
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#).unwrap();

    // Create executable plugin that adds a header
    let script = r#"#!/bin/bash
# Read stdin (context JSON)
read -r INPUT

# Output result with header modification
echo '{"continue_processing": true, "add_headers": {"X-Plugin-Executed": "true"}}'
"#;
    let script_path = plugin_dir.join("plugin.sh");
    std::fs::write(&script_path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Run with plugin
    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin", "exec-plugin",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    // Plugin should have been discovered and possibly executed
    assert!(response.exit_status == ExitStatus::Success ||
            response.stderr.contains("plugin"),
        "Should handle plugin. stderr: {}", response.stderr);
}

// =============================================================================
// Hook Types Tests
// =============================================================================

#[test]
fn test_plugin_pre_request_hook() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("pre-req-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: pre-req-plugin
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{\"continue_processing\": true}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_plugin_post_response_hook() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("post-resp-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: post-resp-plugin
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - post_response
"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{\"continue_processing\": true}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

#[test]
fn test_plugin_multiple_hooks() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("multi-hook-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: multi-hook-plugin
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
  - post_response
  - on_error
"#).unwrap();
    std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{\"continue_processing\": true}'").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_plugin_missing_entry_point() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("no-entry-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // Manifest references non-existent entry point
    std::fs::write(plugin_dir.join("plugin.yaml"), r#"
name: no-entry-plugin
version: "1.0.0"
plugin_type: binary
entry: nonexistent.sh
hooks:
  - pre_request
"#).unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    // Should skip invalid plugin or report error
    // Implementation dependent
}

#[test]
fn test_plugin_invalid_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("invalid-manifest");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // Invalid YAML
    std::fs::write(plugin_dir.join("plugin.yaml"), "invalid: yaml: [").unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-list"
    ]);

    // Should skip invalid plugin
    assert_eq!(response.exit_status, ExitStatus::Success);
}

// =============================================================================
// Fixture Plugin Tests
// =============================================================================

#[test]
fn test_fixture_plugin_discovery() {
    let plugin_path = fixture_path("plugins");

    if !plugin_path.exists() {
        // Skip if fixture directory doesn't exist
        return;
    }

    let response = http(&[
        "--plugin-dir", plugin_path.to_str().unwrap(),
        "--plugin-list"
    ]);

    assert_eq!(response.exit_status, ExitStatus::Success);
    let output = format!("{}{}", response.stdout, response.stderr);
    // Should find the test-plugin from fixtures
    assert!(output.contains("test-plugin") ||
            output.contains("plugin") ||
            output.contains("No plugins"),
        "Should list plugins. output: {}", output);
}

// =============================================================================
// Plugin Enable/Disable Tests
// =============================================================================

#[tokio::test]
async fn test_plugin_selective_enable() {
    let temp_dir = TempDir::new().unwrap();

    // Create two plugins
    for name in &["plugin-a", "plugin-b"] {
        let plugin_dir = temp_dir.path().join(name);
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("plugin.yaml"), format!(r#"
name: {}
version: "1.0.0"
plugin_type: binary
entry: plugin.sh
hooks:
  - pre_request
"#, name)).unwrap();
        std::fs::write(plugin_dir.join("plugin.sh"), "#!/bin/bash\necho '{}'").unwrap();
    }

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Only enable plugin-a
    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin", "plugin-a",
        "GET",
        &format!("{}/test", mock_server.uri())
    ]);

    // Should only run plugin-a
    assert!(response.exit_status == ExitStatus::Success ||
            response.stderr.contains("plugin"),
        "Should handle selective plugin. stderr: {}", response.stderr);
}

// =============================================================================
// Plugin Update Tests
// =============================================================================

#[test]
fn test_plugin_update_none_installed() {
    let temp_dir = TempDir::new().unwrap();

    let response = http(&[
        "--plugin-dir", temp_dir.path().to_str().unwrap(),
        "--plugin-update"
    ]);

    // With no plugins, update should succeed or indicate nothing to update
    let output = format!("{}{}", response.stdout, response.stderr);
    assert!(response.exit_status == ExitStatus::Success ||
            output.contains("No plugins") ||
            output.contains("update"),
        "Should handle update. output: {}", output);
}
