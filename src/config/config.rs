//! Config file handling

use std::path::PathBuf;
use crate::context::Environment;
use crate::errors::QuicpulseError;

/// Hook configuration for running scripts at specific points
#[derive(Debug, Clone, Default)]
pub struct HooksConfig {
    /// Script to run before each HTTP request (can modify request)
    pub pre_request: Option<HookDef>,
    /// Script to run after each HTTP response (for logging/metrics)
    pub post_request: Option<HookDef>,
    /// Script to run on request failure
    pub on_error: Option<HookDef>,
    /// Script to run at workflow start
    pub on_workflow_start: Option<HookDef>,
    /// Script to run at workflow end
    pub on_workflow_end: Option<HookDef>,
}

/// A hook definition - either a file path or inline code
#[derive(Debug, Clone)]
pub enum HookDef {
    /// Path to a Rune script file
    File(PathBuf),
    /// Inline Rune code
    Inline(String),
}

/// QuicPulse configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub config_dir: PathBuf,
    pub default_options: Vec<String>,
    pub disable_update_warnings: bool,
    pub hooks: HooksConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_dir: Self::default_config_dir(),
            default_options: Vec::new(),
            disable_update_warnings: false,
            hooks: HooksConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from the config file (TOML format)
    pub fn load(_env: &Environment) -> Result<Self, QuicpulseError> {
        let config_dir = Self::default_config_dir();
        let config_file = config_dir.join("config.toml");

        if !config_file.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_file)
            .map_err(|e| QuicpulseError::Config(format!("Failed to read config: {}", e)))?;

        let toml_value: toml::Value = toml::from_str(&content)
            .map_err(|e| QuicpulseError::Config(format!("Invalid config TOML: {}", e)))?;

        let default_options = toml_value
            .get("defaults")
            .and_then(|d| d.get("options"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let disable_update_warnings = toml_value
            .get("defaults")
            .and_then(|d| d.get("disable_update_warnings"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse hooks configuration
        let hooks = Self::parse_hooks(&toml_value, &config_dir);

        Ok(Self {
            config_dir,
            default_options,
            disable_update_warnings,
            hooks,
        })
    }

    /// Parse hooks from TOML config
    fn parse_hooks(toml: &toml::Value, config_dir: &PathBuf) -> HooksConfig {
        let hooks_section = match toml.get("hooks") {
            Some(h) => h,
            None => return HooksConfig::default(),
        };

        HooksConfig {
            pre_request: Self::parse_hook_def(hooks_section, "pre_request", config_dir),
            post_request: Self::parse_hook_def(hooks_section, "post_request", config_dir),
            on_error: Self::parse_hook_def(hooks_section, "on_error", config_dir),
            on_workflow_start: Self::parse_hook_def(hooks_section, "on_workflow_start", config_dir),
            on_workflow_end: Self::parse_hook_def(hooks_section, "on_workflow_end", config_dir),
        }
    }

    /// Parse a single hook definition
    fn parse_hook_def(hooks: &toml::Value, name: &str, config_dir: &PathBuf) -> Option<HookDef> {
        // Try file path first (hooks.pre_request = "path/to/script.rune")
        if let Some(path) = hooks.get(name).and_then(|v| v.as_str()) {
            let path = PathBuf::from(path);
            // Resolve relative paths against config dir
            let full_path = if path.is_absolute() {
                path
            } else {
                config_dir.join(path)
            };
            return Some(HookDef::File(full_path));
        }

        // Try inline code (hooks.pre_request_inline.code = "...")
        let inline_name = format!("{}_inline", name);
        if let Some(inline) = hooks.get(&inline_name) {
            if let Some(code) = inline.get("code").and_then(|v| v.as_str()) {
                return Some(HookDef::Inline(code.to_string()));
            }
        }

        None
    }

    /// Get the default config directory
    fn default_config_dir() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("quicpulse"))
            .unwrap_or_else(|| PathBuf::from(".quicpulse"))
    }

    /// Get the sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.config_dir.join("sessions")
    }

    /// Get the version info file path
    pub fn version_info_file(&self) -> PathBuf {
        self.config_dir.join("version_info.json")
    }
}
