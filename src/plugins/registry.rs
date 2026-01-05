//! Plugin registry for discovering and installing plugins

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::errors::QuicpulseError;

/// Registry entry for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Author
    pub author: Option<String>,

    /// Repository URL
    pub repository: String,

    /// Download URL
    pub download_url: Option<String>,

    /// Keywords
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Downloads count
    #[serde(default)]
    pub downloads: u64,

    /// Rating
    #[serde(default)]
    pub rating: f32,

    /// Last updated
    pub updated_at: Option<String>,
}

/// Plugin registry client
pub struct PluginRegistry {
    /// Registry URL
    registry_url: String,

    /// HTTP client
    client: reqwest::Client,

    /// Local cache directory
    cache_dir: PathBuf,
}

impl PluginRegistry {
    /// Default registry URL
    pub const DEFAULT_REGISTRY: &'static str = "https://plugins.quicpulse.io";

    /// Create a new registry client
    pub fn new() -> Self {
        Self::with_url(Self::DEFAULT_REGISTRY)
    }

    /// Create with custom registry URL
    pub fn with_url(url: &str) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("quicpulse")
            .join("plugins");

        Self {
            registry_url: url.to_string(),
            client: reqwest::Client::new(),
            cache_dir,
        }
    }

    /// Search for plugins
    pub async fn search(&self, query: &str) -> Result<Vec<PluginEntry>, QuicpulseError> {
        let url = format!("{}/api/search?q={}", self.registry_url, urlencoding::encode(query));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(QuicpulseError::Request)?;

        if !response.status().is_success() {
            return Err(QuicpulseError::Config(format!(
                "Registry search failed: {}",
                response.status()
            )));
        }

        response.json().await
            .map_err(QuicpulseError::Request)
    }

    /// Get plugin details
    pub async fn get(&self, name: &str) -> Result<PluginEntry, QuicpulseError> {
        let url = format!("{}/api/plugins/{}", self.registry_url, urlencoding::encode(name));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(QuicpulseError::Request)?;

        if !response.status().is_success() {
            return Err(QuicpulseError::Config(format!(
                "Plugin not found: {}",
                name
            )));
        }

        response.json().await
            .map_err(QuicpulseError::Request)
    }

    /// Install a plugin from the registry
    pub async fn install(&self, name: &str, dest: &PathBuf) -> Result<(), QuicpulseError> {
        let entry = self.get(name).await?;

        // Clone from repository
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", &entry.repository])
            .arg(dest)
            .status()
            .map_err(|e| QuicpulseError::Config(format!("Failed to clone plugin: {}", e)))?;

        if !status.success() {
            return Err(QuicpulseError::Config("Failed to clone plugin repository".to_string()));
        }

        Ok(())
    }

    /// List popular plugins
    pub async fn list_popular(&self, limit: usize) -> Result<Vec<PluginEntry>, QuicpulseError> {
        let url = format!("{}/api/plugins?sort=downloads&limit={}", self.registry_url, limit);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(QuicpulseError::Request)?;

        if !response.status().is_success() {
            return Err(QuicpulseError::Config(format!(
                "Failed to fetch plugins: {}",
                response.status()
            )));
        }

        response.json().await
            .map_err(QuicpulseError::Request)
    }

    /// Get installed plugins directory
    pub fn plugins_dir() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("quicpulse").join("plugins")
        } else if let Some(home) = dirs::home_dir() {
            home.join(".quicpulse").join("plugins")
        } else {
            PathBuf::from("plugins")
        }
    }

    /// Ensure plugins directory exists
    pub fn ensure_plugins_dir() -> Result<PathBuf, QuicpulseError> {
        let dir = Self::plugins_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(QuicpulseError::Io)?;
        }
        Ok(dir)
    }

    /// Uninstall a plugin
    pub fn uninstall(name: &str) -> Result<(), QuicpulseError> {
        let plugin_dir = Self::plugins_dir().join(name);

        if !plugin_dir.exists() {
            return Err(QuicpulseError::Config(format!(
                "Plugin not installed: {}",
                name
            )));
        }

        std::fs::remove_dir_all(&plugin_dir)
            .map_err(QuicpulseError::Io)?;

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugins_dir() {
        let dir = PluginRegistry::plugins_dir();
        // Should return a valid path
        assert!(!dir.as_os_str().is_empty());
    }
}
