//! Plugin manager

use std::collections::HashMap;
use crate::errors::QuicpulseError;
use super::config::PluginsConfig;
use super::hooks::{HookContext, HookResult, PluginHook};
use super::loader::{LoadedPlugin, PluginLoader, execute_script_hook};

/// Plugin manager
pub struct PluginManager {
    /// Loaded plugins
    plugins: Vec<LoadedPlugin>,

    /// Plugin configurations
    config: PluginsConfig,

    /// Hooks mapped to plugins
    hook_map: HashMap<PluginHook, Vec<usize>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            config: PluginsConfig::default(),
            hook_map: HashMap::new(),
        }
    }

    /// Create with configuration
    pub fn with_config(config: PluginsConfig) -> Self {
        Self {
            plugins: Vec::new(),
            config,
            hook_map: HashMap::new(),
        }
    }

    /// Load plugins from default directories
    pub fn load_default(&mut self) -> Result<usize, QuicpulseError> {
        let loader = PluginLoader::default();
        let plugins = loader.discover()?;
        let count = plugins.len();

        for plugin in plugins {
            self.register_plugin(plugin);
        }

        Ok(count)
    }

    /// Load plugins from a specific directory
    pub fn load_from_dir(&mut self, dir: &std::path::Path) -> Result<usize, QuicpulseError> {
        let mut loader = PluginLoader::new();
        loader.add_search_path(dir);
        let plugins = loader.discover()?;
        let count = plugins.len();

        for plugin in plugins {
            self.register_plugin(plugin);
        }

        Ok(count)
    }

    /// Register a loaded plugin
    pub fn register_plugin(&mut self, plugin: LoadedPlugin) {
        let name = plugin.name().to_string();

        // Check if plugin is enabled in config
        if !self.config.is_enabled(&name) {
            return;
        }

        let index = self.plugins.len();

        // Map hooks
        for hook in &plugin.hooks {
            self.hook_map.entry(*hook).or_default().push(index);
        }

        self.plugins.push(plugin);
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.iter().find(|p| p.name() == name)
    }

    /// Check if any plugins handle a hook
    pub fn has_hook_handlers(&self, hook: PluginHook) -> bool {
        self.hook_map.get(&hook).map(|v| !v.is_empty()).unwrap_or(false)
    }

    /// Execute all plugins for a hook
    pub async fn execute_hook(
        &self,
        hook: PluginHook,
        context: HookContext,
    ) -> Result<HookResult, QuicpulseError> {
        let plugin_indices = match self.hook_map.get(&hook) {
            Some(indices) => indices,
            None => return Ok(HookResult::ok()),
        };

        let mut current_context = context;
        let mut final_result = HookResult::ok();

        for &index in plugin_indices {
            let plugin = &self.plugins[index];

            let result = execute_script_hook(plugin, hook, &current_context).await?;

            // Check if we should stop processing
            if !result.continue_processing {
                return Ok(result);
            }

            // Merge results
            if let Some(ref url) = result.url {
                final_result.url = Some(url.clone());
                current_context.url = Some(url.clone());
            }

            if let Some(ref method) = result.method {
                final_result.method = Some(method.clone());
                current_context.method = Some(method.clone());
            }

            if let Some(ref body) = result.body {
                final_result.body = Some(body.clone());
                current_context.request_body = Some(body.clone());
            }

            // Merge headers
            for (key, value) in &result.headers {
                final_result.headers.insert(key.clone(), value.clone());
                current_context.request_headers.insert(key.clone(), value.clone());
            }

            // Merge remove_headers
            final_result.remove_headers.extend(result.remove_headers.clone());

            // Merge data
            for (key, value) in &result.data {
                final_result.data.insert(key.clone(), value.clone());
                current_context.data.insert(key.clone(), value.clone());
            }

            // Set output if provided
            if result.output.is_some() {
                final_result.output = result.output;
            }
        }

        Ok(final_result)
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<(&str, &str, bool)> {
        self.plugins.iter()
            .map(|p| {
                let name = p.name();
                let version = p.version();
                let enabled = self.config.is_enabled(name);
                (name, version, enabled)
            })
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let manager = PluginManager::new();
        assert!(manager.plugins.is_empty());
    }

    #[test]
    fn test_has_hook_handlers() {
        let manager = PluginManager::new();
        assert!(!manager.has_hook_handlers(PluginHook::PreRequest));
    }
}
