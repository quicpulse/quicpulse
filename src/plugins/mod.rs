//! Plugin ecosystem for QuicPulse
//!
//! Supports loading and executing plugins that extend QuicPulse functionality.
//! Plugins can:
//! - Add custom authentication methods
//! - Transform requests before sending
//! - Transform responses after receiving
//! - Add custom output formatters
//! - Provide custom commands

pub mod config;
pub mod hooks;
pub mod loader;
pub mod manager;
pub mod registry;
pub mod bundled;

pub use config::PluginConfig;
pub use hooks::{HookContext, HookResult, PluginHook};
pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use registry::PluginRegistry;
