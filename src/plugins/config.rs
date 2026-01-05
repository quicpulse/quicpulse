//! Plugin configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::errors::QuicpulseError;

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    #[serde(default)]
    pub description: String,

    /// Plugin author
    #[serde(default)]
    pub author: Option<String>,

    /// Plugin homepage/repository URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Minimum QuicPulse version required
    #[serde(default)]
    pub min_version: Option<String>,

    /// Plugin type
    #[serde(default)]
    pub plugin_type: PluginType,

    /// Entry point (script file or binary)
    pub entry: String,

    /// Hooks this plugin provides
    #[serde(default)]
    pub hooks: Vec<String>,

    /// Plugin-specific configuration schema
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,

    /// Dependencies on other plugins
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Keywords for searching
    #[serde(default)]
    pub keywords: Vec<String>,
}

/// Plugin type
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Script-based plugin (Rune/Lua/JavaScript)
    #[default]
    Script,
    /// Native binary plugin
    Binary,
    /// WASM plugin
    Wasm,
    /// Bundled plugin (compiled into the binary, disabled by default)
    Bundled,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Whether the plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Plugin-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Override hooks
    #[serde(default)]
    pub hooks: Option<Vec<String>>,
}

fn default_true() -> bool {
    true
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            config: HashMap::new(),
            hooks: None,
        }
    }
}

impl PluginManifest {
    /// Load manifest from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, QuicpulseError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(QuicpulseError::Io)?;

        let ext = path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        match ext {
            "yaml" | "yml" => {
                serde_yaml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugin manifest: {}", e)))
            }
            "toml" => {
                toml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugin manifest: {}", e)))
            }
            _ => {
                serde_json::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugin manifest: {}", e)))
            }
        }
    }

    /// Get the full entry path relative to plugin directory
    pub fn entry_path(&self, plugin_dir: &Path) -> PathBuf {
        plugin_dir.join(&self.entry)
    }
}

/// Global plugins configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsConfig {
    /// Plugin directory
    #[serde(default)]
    pub plugin_dir: Option<PathBuf>,

    /// Individual plugin configurations
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,

    /// Global hook ordering
    #[serde(default)]
    pub hook_order: HashMap<String, Vec<String>>,
}

impl PluginsConfig {
    /// Load from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, QuicpulseError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(QuicpulseError::Io)?;

        let ext = path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        match ext {
            "yaml" | "yml" => {
                serde_yaml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugins config: {}", e)))
            }
            "toml" => {
                toml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugins config: {}", e)))
            }
            _ => {
                serde_json::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugins config: {}", e)))
            }
        }
    }

    /// Get plugin config by name
    pub fn get(&self, name: &str) -> Option<&PluginConfig> {
        self.plugins.get(name)
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.plugins.get(name)
            .map(|c| c.enabled)
            .unwrap_or(true)
    }

    /// Check if a bundled plugin is enabled (default: false for bundled plugins)
    pub fn is_bundled_enabled(&self, name: &str) -> bool {
        self.plugins.get(name)
            .map(|c| c.enabled)
            .unwrap_or(false) // Bundled plugins are disabled by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest_parse() {
        let yaml = r#"
name: my-plugin
version: 1.0.0
description: A test plugin
entry: main.rn
hooks:
  - pre_request
  - post_response
"#;
        let manifest: PluginManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "my-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.hooks.len(), 2);
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.enabled);
        assert!(config.config.is_empty());
    }
}
