//! Plugin loader

use std::path::{Path, PathBuf};
use crate::errors::QuicpulseError;
use super::config::{PluginManifest, PluginType};
use super::hooks::{HookContext, HookResult, PluginHook};

/// A loaded plugin
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    /// Plugin manifest
    pub manifest: PluginManifest,

    /// Plugin directory
    pub directory: PathBuf,

    /// Parsed hooks this plugin provides
    pub hooks: Vec<PluginHook>,
}

impl LoadedPlugin {
    /// Check if this plugin handles a specific hook
    pub fn handles_hook(&self, hook: PluginHook) -> bool {
        self.hooks.contains(&hook)
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// Get plugin version
    pub fn version(&self) -> &str {
        &self.manifest.version
    }
}

/// Plugin loader
pub struct PluginLoader {
    /// Plugin search paths
    search_paths: Vec<PathBuf>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    /// Add a search path
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Get the search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Get default plugin directories
    pub fn default_plugin_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // User plugins directory
        if let Some(config_dir) = dirs::config_dir() {
            dirs.push(config_dir.join("quicpulse").join("plugins"));
        }

        // Home directory plugins
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".quicpulse").join("plugins"));
        }

        // Local plugins directory
        dirs.push(PathBuf::from("plugins"));

        dirs
    }

    /// Discover all plugins in search paths
    pub fn discover(&self) -> Result<Vec<LoadedPlugin>, QuicpulseError> {
        let mut plugins = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            // Each subdirectory could be a plugin
            let entries = std::fs::read_dir(search_path)
                .map_err(QuicpulseError::Io)?;

            for entry in entries {
                let entry = entry.map_err(QuicpulseError::Io)?;
                let path = entry.path();

                if path.is_dir() {
                    if let Ok(plugin) = self.load_plugin(&path) {
                        plugins.push(plugin);
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// Load a plugin from a directory
    pub fn load_plugin<P: AsRef<Path>>(&self, dir: P) -> Result<LoadedPlugin, QuicpulseError> {
        let dir = dir.as_ref();

        // Look for manifest file
        let manifest_path = Self::find_manifest(dir)?;
        let manifest = PluginManifest::load(&manifest_path)?;

        // Verify entry point exists
        let entry_path = manifest.entry_path(dir);
        if !entry_path.exists() {
            return Err(QuicpulseError::Config(format!(
                "Plugin entry point not found: {:?}",
                entry_path
            )));
        }

        // Parse hooks
        let hooks: Vec<PluginHook> = manifest.hooks.iter()
            .filter_map(|h| PluginHook::from_str(h))
            .collect();

        Ok(LoadedPlugin {
            manifest,
            directory: dir.to_path_buf(),
            hooks,
        })
    }

    /// Find manifest file in plugin directory
    fn find_manifest(dir: &Path) -> Result<PathBuf, QuicpulseError> {
        // Try different manifest file names
        let names = [
            "plugin.yaml",
            "plugin.yml",
            "plugin.json",
            "plugin.toml",
            "manifest.yaml",
            "manifest.yml",
            "manifest.json",
            "manifest.toml",
        ];

        for name in &names {
            let path = dir.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(QuicpulseError::Config(format!(
            "No plugin manifest found in {:?}",
            dir
        )))
    }

    /// Load a plugin from a git URL
    pub async fn load_from_git(&self, url: &str, dest: &Path) -> Result<LoadedPlugin, QuicpulseError> {
        // Clone the repository
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url])
            .arg(dest)
            .status()
            .map_err(|e| QuicpulseError::Config(format!("Failed to clone plugin: {}", e)))?;

        if !status.success() {
            return Err(QuicpulseError::Config("Failed to clone plugin repository".to_string()));
        }

        self.load_plugin(dest)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        let mut loader = Self::new();
        for dir in Self::default_plugin_dirs() {
            loader.add_search_path(dir);
        }
        loader
    }
}

/// Execute a script-based plugin hook
pub async fn execute_script_hook(
    plugin: &LoadedPlugin,
    hook: PluginHook,
    context: &HookContext,
) -> Result<HookResult, QuicpulseError> {
    let entry_path = plugin.manifest.entry_path(&plugin.directory);
    let ext = entry_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match (plugin.manifest.plugin_type.clone(), ext) {
        (PluginType::Script, "rn") => {
            // Rune script
            execute_rune_script(&entry_path, hook, context).await
        }
        (PluginType::Binary, _) => {
            // Binary plugin - execute as subprocess
            execute_binary_plugin(&entry_path, hook, context).await
        }
        _ => {
            Err(QuicpulseError::Config(format!(
                "Unsupported plugin type: {:?} / {}",
                plugin.manifest.plugin_type, ext
            )))
        }
    }
}

/// Execute a Rune script plugin
async fn execute_rune_script(
    path: &Path,
    hook: PluginHook,
    context: &HookContext,
) -> Result<HookResult, QuicpulseError> {
    // Read the script
    let script = std::fs::read_to_string(path)
        .map_err(QuicpulseError::Io)?;

    // For now, return a default result
    // In a full implementation, this would use the rune VM to execute the script
    eprintln!("Plugin script execution not fully implemented yet: {:?}", path);
    eprintln!("Hook: {}", hook.as_str());

    Ok(HookResult::ok())
}

/// Execute a binary plugin
async fn execute_binary_plugin(
    path: &Path,
    hook: PluginHook,
    context: &HookContext,
) -> Result<HookResult, QuicpulseError> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;

    // Serialize context to JSON
    let context_json = serde_json::to_string(context)
        .map_err(|e| QuicpulseError::Config(format!("Failed to serialize context: {}", e)))?;

    // Execute the binary with hook name as argument
    let mut child = tokio::process::Command::new(path)
        .arg(hook.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| QuicpulseError::Config(format!("Failed to execute plugin: {}", e)))?;

    // Write context to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(context_json.as_bytes()).await
            .map_err(|e| QuicpulseError::Config(format!("Failed to write to plugin stdin: {}", e)))?;
    }

    // Wait for output
    let output = child.wait_with_output().await
        .map_err(|e| QuicpulseError::Config(format!("Failed to get plugin output: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QuicpulseError::Config(format!("Plugin failed: {}", stderr)));
    }

    // Parse output as HookResult
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(HookResult::ok());
    }

    serde_json::from_str(&stdout)
        .map_err(|e| QuicpulseError::Config(format!("Failed to parse plugin output: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_plugin_dirs() {
        let dirs = PluginLoader::default_plugin_dirs();
        // Should have at least the local plugins directory
        assert!(!dirs.is_empty());
    }

    #[test]
    fn test_loaded_plugin_handles_hook() {
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
                hooks: vec!["pre_request".to_string()],
                config_schema: None,
                dependencies: Vec::new(),
                keywords: Vec::new(),
            },
            directory: PathBuf::from("/tmp"),
            hooks: vec![PluginHook::PreRequest],
        };

        assert!(plugin.handles_hook(PluginHook::PreRequest));
        assert!(!plugin.handles_hook(PluginHook::PostResponse));
    }
}
