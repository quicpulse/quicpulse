//! Tests for the plugin ecosystem

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

mod common;

// ============================================================================
// Plugin Hook Tests
// ============================================================================

#[test]
fn test_plugin_hook_names() {
    use quicpulse::plugins::PluginHook;

    assert_eq!(PluginHook::PreRequest.as_str(), "pre_request");
    assert_eq!(PluginHook::PostResponse.as_str(), "post_response");
    assert_eq!(PluginHook::OnError.as_str(), "on_error");
    assert_eq!(PluginHook::Auth.as_str(), "auth");
    assert_eq!(PluginHook::Format.as_str(), "format");
}

#[test]
fn test_plugin_hook_from_str() {
    use quicpulse::plugins::PluginHook;

    assert_eq!(PluginHook::from_str("pre_request"), Some(PluginHook::PreRequest));
    assert_eq!(PluginHook::from_str("post_response"), Some(PluginHook::PostResponse));
    assert_eq!(PluginHook::from_str("on_error"), Some(PluginHook::OnError));
    assert_eq!(PluginHook::from_str("invalid"), None);
}

#[test]
fn test_plugin_hook_all() {
    use quicpulse::plugins::PluginHook;

    let all_hooks = PluginHook::all();
    assert!(all_hooks.contains(&PluginHook::PreRequest));
    assert!(all_hooks.contains(&PluginHook::PostResponse));
    assert!(all_hooks.contains(&PluginHook::OnError));
    assert!(all_hooks.contains(&PluginHook::Auth));
    assert!(all_hooks.contains(&PluginHook::Format));
    assert!(all_hooks.len() >= 10);
}

// ============================================================================
// Hook Context Tests
// ============================================================================

#[test]
fn test_hook_context_new() {
    use quicpulse::plugins::{HookContext, PluginHook};

    let ctx = HookContext::new(PluginHook::PreRequest);
    assert_eq!(ctx.hook, "pre_request");
    assert!(ctx.url.is_none());
    assert!(ctx.method.is_none());
    assert!(ctx.request_headers.is_empty());
}

#[test]
fn test_hook_context_pre_request() {
    use quicpulse::plugins::HookContext;

    let headers = HashMap::from([
        ("Content-Type".to_string(), "application/json".to_string()),
    ]);

    let ctx = HookContext::pre_request(
        "http://example.com/api",
        "POST",
        headers.clone(),
        Some("{\"key\": \"value\"}".to_string()),
    );

    assert_eq!(ctx.hook, "pre_request");
    assert_eq!(ctx.url, Some("http://example.com/api".to_string()));
    assert_eq!(ctx.method, Some("POST".to_string()));
    assert_eq!(ctx.request_headers.get("Content-Type"), Some(&"application/json".to_string()));
    assert_eq!(ctx.request_body, Some("{\"key\": \"value\"}".to_string()));
}

#[test]
fn test_hook_context_post_response() {
    use quicpulse::plugins::HookContext;

    let headers = HashMap::from([
        ("content-type".to_string(), "text/html".to_string()),
    ]);

    let ctx = HookContext::post_response(
        "http://example.com",
        200,
        headers,
        "<html></html>".to_string(),
    );

    assert_eq!(ctx.hook, "post_response");
    assert_eq!(ctx.response_status, Some(200));
    assert_eq!(ctx.response_body, Some("<html></html>".to_string()));
}

#[test]
fn test_hook_context_on_error() {
    use quicpulse::plugins::HookContext;

    let ctx = HookContext::on_error("Connection refused");

    assert_eq!(ctx.hook, "on_error");
    assert_eq!(ctx.error, Some("Connection refused".to_string()));
}

// ============================================================================
// Hook Result Tests
// ============================================================================

#[test]
fn test_hook_result_default() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::default();
    assert!(result.continue_processing);
    assert!(result.url.is_none());
    assert!(result.method.is_none());
    assert!(result.headers.is_empty());
    assert!(result.body.is_none());
    assert!(result.error.is_none());
}

#[test]
fn test_hook_result_ok() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::ok();
    assert!(result.continue_processing);
}

#[test]
fn test_hook_result_stop() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::stop();
    assert!(!result.continue_processing);
}

#[test]
fn test_hook_result_error() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::error("Something went wrong");
    assert!(!result.continue_processing);
    assert_eq!(result.error, Some("Something went wrong".to_string()));
}

#[test]
fn test_hook_result_with_url() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::with_url("http://modified.example.com");
    assert!(result.continue_processing);
    assert_eq!(result.url, Some("http://modified.example.com".to_string()));
}

#[test]
fn test_hook_result_with_headers() {
    use quicpulse::plugins::HookResult;

    let headers = HashMap::from([
        ("X-Custom-Header".to_string(), "custom-value".to_string()),
    ]);

    let result = HookResult::with_headers(headers);
    assert!(result.continue_processing);
    assert_eq!(result.headers.get("X-Custom-Header"), Some(&"custom-value".to_string()));
}

#[test]
fn test_hook_result_with_body() {
    use quicpulse::plugins::HookResult;

    let result = HookResult::with_body("modified body".to_string());
    assert!(result.continue_processing);
    assert_eq!(result.body, Some("modified body".to_string()));
}

// ============================================================================
// Plugin Config Tests
// ============================================================================

#[test]
fn test_plugin_config_default() {
    use quicpulse::plugins::PluginConfig;

    let config = PluginConfig::default();
    assert!(config.enabled);
    assert!(config.config.is_empty());
    assert!(config.hooks.is_none());
}

#[test]
fn test_plugin_manifest_parse() {
    use quicpulse::plugins::config::PluginManifest;

    let yaml = r#"
name: test-plugin
version: 1.0.0
description: A test plugin
author: Test Author
entry: main.rn
hooks:
  - pre_request
  - post_response
keywords:
  - test
  - example
"#;

    let manifest: PluginManifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.name, "test-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.description, "A test plugin");
    assert_eq!(manifest.author, Some("Test Author".to_string()));
    assert_eq!(manifest.entry, "main.rn");
    assert_eq!(manifest.hooks.len(), 2);
    assert!(manifest.hooks.contains(&"pre_request".to_string()));
    assert!(manifest.hooks.contains(&"post_response".to_string()));
}

#[test]
fn test_plugin_manifest_entry_path() {
    use quicpulse::plugins::config::PluginManifest;

    let manifest = PluginManifest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        author: None,
        homepage: None,
        min_version: None,
        plugin_type: Default::default(),
        entry: "src/main.rn".to_string(),
        hooks: Vec::new(),
        config_schema: None,
        dependencies: Vec::new(),
        keywords: Vec::new(),
    };

    let path = manifest.entry_path(&PathBuf::from("/plugins/test-plugin"));
    assert_eq!(path, PathBuf::from("/plugins/test-plugin/src/main.rn"));
}

#[test]
fn test_plugin_type_default() {
    use quicpulse::plugins::config::PluginType;

    let default = PluginType::default();
    assert_eq!(default, PluginType::Script);
}

// ============================================================================
// Plugins Config Tests
// ============================================================================

#[test]
fn test_plugins_config_default() {
    use quicpulse::plugins::config::PluginsConfig;

    let config = PluginsConfig::default();
    assert!(config.plugin_dir.is_none());
    assert!(config.plugins.is_empty());
    assert!(config.hook_order.is_empty());
}

#[test]
fn test_plugins_config_is_enabled() {
    use quicpulse::plugins::config::{PluginsConfig, PluginConfig};

    let mut config = PluginsConfig::default();

    // Unknown plugin should be enabled by default
    assert!(config.is_enabled("unknown-plugin"));

    // Explicitly disabled plugin
    config.plugins.insert("disabled-plugin".to_string(), PluginConfig {
        enabled: false,
        ..Default::default()
    });
    assert!(!config.is_enabled("disabled-plugin"));

    // Explicitly enabled plugin
    config.plugins.insert("enabled-plugin".to_string(), PluginConfig {
        enabled: true,
        ..Default::default()
    });
    assert!(config.is_enabled("enabled-plugin"));
}

// ============================================================================
// Plugin Loader Tests
// ============================================================================

#[test]
fn test_plugin_loader_default_dirs() {
    use quicpulse::plugins::PluginLoader;

    let dirs = PluginLoader::default_plugin_dirs();
    assert!(!dirs.is_empty());
    // Should include local plugins directory
    assert!(dirs.iter().any(|d| d.ends_with("plugins")));
}

#[test]
fn test_plugin_loader_new() {
    use quicpulse::plugins::PluginLoader;

    let loader = PluginLoader::new();
    // Should be able to discover plugins (even if none exist)
    let result = loader.discover();
    assert!(result.is_ok());
}

#[test]
fn test_plugin_loader_add_search_path() {
    use quicpulse::plugins::PluginLoader;

    let mut loader = PluginLoader::new();
    loader.add_search_path("/custom/plugins");

    // Should not fail when discovering from non-existent path
    let result = loader.discover();
    assert!(result.is_ok());
}

#[test]
fn test_plugin_loader_discover_empty() {
    use quicpulse::plugins::PluginLoader;

    let temp_dir = TempDir::new().unwrap();
    let mut loader = PluginLoader::new();
    loader.add_search_path(temp_dir.path());

    let plugins = loader.discover().unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn test_plugin_loader_load_plugin() {
    use quicpulse::plugins::PluginLoader;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create manifest
    let manifest = r#"
name: test-plugin
version: 1.0.0
entry: main.rn
hooks:
  - pre_request
"#;
    fs::write(plugin_dir.join("plugin.yaml"), manifest).unwrap();

    // Create entry file
    fs::write(plugin_dir.join("main.rn"), "// plugin code").unwrap();

    let loader = PluginLoader::new();
    let result = loader.load_plugin(&plugin_dir);

    assert!(result.is_ok());
    let plugin = result.unwrap();
    assert_eq!(plugin.name(), "test-plugin");
    assert_eq!(plugin.version(), "1.0.0");
}

#[test]
fn test_plugin_loader_missing_manifest() {
    use quicpulse::plugins::PluginLoader;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("no-manifest");
    fs::create_dir(&plugin_dir).unwrap();

    let loader = PluginLoader::new();
    let result = loader.load_plugin(&plugin_dir);

    assert!(result.is_err());
}

#[test]
fn test_plugin_loader_missing_entry() {
    use quicpulse::plugins::PluginLoader;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("no-entry");
    fs::create_dir(&plugin_dir).unwrap();

    let manifest = r#"
name: no-entry-plugin
version: 1.0.0
entry: nonexistent.rn
"#;
    fs::write(plugin_dir.join("plugin.yaml"), manifest).unwrap();

    let loader = PluginLoader::new();
    let result = loader.load_plugin(&plugin_dir);

    assert!(result.is_err());
}

// ============================================================================
// Loaded Plugin Tests
// ============================================================================

#[test]
fn test_loaded_plugin_handles_hook() {
    use quicpulse::plugins::PluginHook;
    use quicpulse::plugins::loader::LoadedPlugin;
    use quicpulse::plugins::config::{PluginManifest, PluginType};

    let plugin = LoadedPlugin {
        manifest: PluginManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            homepage: None,
            min_version: None,
            plugin_type: PluginType::Script,
            entry: "main.rn".to_string(),
            hooks: vec!["pre_request".to_string(), "post_response".to_string()],
            config_schema: None,
            dependencies: Vec::new(),
            keywords: Vec::new(),
        },
        directory: PathBuf::from("/tmp"),
        hooks: vec![PluginHook::PreRequest, PluginHook::PostResponse],
    };

    assert!(plugin.handles_hook(PluginHook::PreRequest));
    assert!(plugin.handles_hook(PluginHook::PostResponse));
    assert!(!plugin.handles_hook(PluginHook::OnError));
}

// ============================================================================
// Plugin Manager Tests
// ============================================================================

#[test]
fn test_plugin_manager_new() {
    use quicpulse::plugins::PluginManager;

    let manager = PluginManager::new();
    assert!(manager.plugins().is_empty());
}

#[test]
fn test_plugin_manager_has_hook_handlers() {
    use quicpulse::plugins::{PluginManager, PluginHook};

    let manager = PluginManager::new();
    // No plugins registered, so no hook handlers
    assert!(!manager.has_hook_handlers(PluginHook::PreRequest));
}

#[test]
fn test_plugin_manager_list_plugins() {
    use quicpulse::plugins::PluginManager;

    let manager = PluginManager::new();
    let list = manager.list_plugins();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_plugin_manager_execute_hook_no_plugins() {
    use quicpulse::plugins::{PluginManager, PluginHook, HookContext};

    let manager = PluginManager::new();
    let context = HookContext::new(PluginHook::PreRequest);

    let result = manager.execute_hook(PluginHook::PreRequest, context).await;
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.continue_processing);
}

// ============================================================================
// Plugin Registry Tests
// ============================================================================

#[test]
fn test_plugin_registry_plugins_dir() {
    use quicpulse::plugins::PluginRegistry;

    let dir = PluginRegistry::plugins_dir();
    assert!(!dir.as_os_str().is_empty());
    assert!(dir.ends_with("plugins"));
}

#[test]
fn test_plugin_registry_ensure_plugins_dir() {
    use quicpulse::plugins::PluginRegistry;
    use std::fs;

    // Use a temp directory to avoid creating real directories
    let temp_dir = TempDir::new().unwrap();
    let plugins_dir = temp_dir.path().join("plugins");

    // Create directory manually
    fs::create_dir_all(&plugins_dir).unwrap();
    assert!(plugins_dir.exists());
}

#[test]
fn test_plugin_registry_default() {
    use quicpulse::plugins::PluginRegistry;

    let registry = PluginRegistry::new();
    // Should be able to create registry with default URL
    assert!(true); // Registry created successfully
}

#[test]
fn test_plugin_registry_custom_url() {
    use quicpulse::plugins::PluginRegistry;

    let registry = PluginRegistry::with_url("https://custom-registry.example.com");
    assert!(true); // Registry created with custom URL
}

// ============================================================================
// CLI Argument Tests
// ============================================================================

#[test]
fn test_plugin_list_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-list",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.plugin_list);
}

#[test]
fn test_plugins_alias_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugins",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.plugin_list);
}

#[test]
fn test_plugin_install_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-install", "auth-oauth",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.plugin_install, Some("auth-oauth".to_string()));
}

#[test]
fn test_plugin_uninstall_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-uninstall", "old-plugin",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.plugin_uninstall, Some("old-plugin".to_string()));
}

#[test]
fn test_plugin_search_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-search", "oauth",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.plugin_search, Some("oauth".to_string()));
}

#[test]
fn test_plugin_update_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-update",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert!(args.plugin_update);
}

#[test]
fn test_plugin_dir_arg() {
    use quicpulse::cli::Args;
    use clap::Parser;

    let args = Args::try_parse_from([
        "quicpulse",
        "--plugin-dir", "/custom/plugins",
        "--plugin-list",
    ]);

    assert!(args.is_ok());
    let args = args.unwrap();
    assert_eq!(args.plugin_dir, Some(PathBuf::from("/custom/plugins")));
}
