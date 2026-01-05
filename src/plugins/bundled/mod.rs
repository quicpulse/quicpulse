//! Bundled plugins for QuicPulse
//!
//! These plugins are compiled into the binary but are disabled by default.
//! Users can enable them via the plugins configuration file.

#[cfg(feature = "javascript")]
pub mod javascript;

use crate::plugins::config::{PluginManifest, PluginConfig, PluginType};
use std::collections::HashMap;

/// Information about a bundled plugin
pub struct BundledPlugin {
    /// The plugin manifest
    pub manifest: PluginManifest,
    /// Default configuration (disabled by default)
    pub default_config: PluginConfig,
}

/// Get all bundled plugins
pub fn get_bundled_plugins() -> HashMap<String, BundledPlugin> {
    let mut plugins = HashMap::new();

    #[cfg(feature = "javascript")]
    {
        let js_plugin = javascript::get_plugin();
        plugins.insert(js_plugin.manifest.name.clone(), js_plugin);
    }

    plugins
}

/// Check if a bundled plugin is available
pub fn is_available(name: &str) -> bool {
    match name {
        #[cfg(feature = "javascript")]
        "quicpulse-javascript" => true,
        _ => false,
    }
}

/// Get the list of available bundled plugin names
pub fn available_plugins() -> Vec<&'static str> {
    let mut plugins = Vec::new();

    #[cfg(feature = "javascript")]
    plugins.push("quicpulse-javascript");

    plugins
}
