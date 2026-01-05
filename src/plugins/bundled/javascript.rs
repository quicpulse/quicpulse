//! JavaScript bundled plugin
//!
//! Provides JavaScript scripting support via QuickJS.
//! This plugin is disabled by default and must be explicitly enabled.

use crate::plugins::config::{PluginManifest, PluginConfig, PluginType};
use super::BundledPlugin;
use std::collections::HashMap;

/// Plugin name constant
pub const PLUGIN_NAME: &str = "quicpulse-javascript";

/// Get the JavaScript plugin manifest and default config
pub fn get_plugin() -> BundledPlugin {
    BundledPlugin {
        manifest: manifest(),
        default_config: default_config(),
    }
}

/// Get the plugin manifest
pub fn manifest() -> PluginManifest {
    PluginManifest {
        name: PLUGIN_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "JavaScript scripting support via QuickJS".to_string(),
        author: Some("QuicPulse Team".to_string()),
        homepage: Some("https://github.com/quicpulse/quicpulse".to_string()),
        min_version: None,
        plugin_type: PluginType::Bundled,
        entry: "builtin:javascript".to_string(),
        hooks: vec![
            "pre_request".to_string(),
            "post_response".to_string(),
            "transform_request".to_string(),
            "transform_response".to_string(),
            "assertion".to_string(),
        ],
        config_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "memory_limit_mb": {
                    "type": "integer",
                    "default": 64,
                    "description": "Maximum memory limit for JavaScript runtime in MB"
                },
                "timeout_ms": {
                    "type": "integer",
                    "default": 30000,
                    "description": "Maximum execution timeout in milliseconds"
                }
            }
        })),
        dependencies: vec![],
        keywords: vec![
            "javascript".to_string(),
            "js".to_string(),
            "quickjs".to_string(),
            "scripting".to_string(),
        ],
    }
}

/// Get the default plugin configuration
pub fn default_config() -> PluginConfig {
    PluginConfig {
        enabled: false, // Disabled by default
        config: HashMap::new(),
        hooks: None,
    }
}

/// Check if the JavaScript plugin is enabled in the given config
pub fn is_enabled(plugins_config: &crate::plugins::config::PluginsConfig) -> bool {
    plugins_config.is_bundled_enabled(PLUGIN_NAME)
}
