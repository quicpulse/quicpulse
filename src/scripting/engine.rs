//! Multi-language script engine abstraction
//!
//! Provides a unified interface for executing scripts in different languages
//! (Rune, JavaScript) while maintaining a consistent API.

use crate::errors::QuicpulseError;
use super::context::ScriptContext;
use super::runtime::{ScriptEngine, ScriptResult};
use super::detection::ScriptType;

#[cfg(feature = "javascript")]
use super::js::JsScriptEngine;

/// Multi-language script engine that dispatches to the appropriate runtime
pub struct MultiScriptEngine {
    /// Rune script engine (always available)
    rune_engine: ScriptEngine,

    /// JavaScript engine (optional, enabled via plugin)
    #[cfg(feature = "javascript")]
    js_engine: Option<JsScriptEngine>,

    /// Whether JavaScript is enabled
    #[cfg(feature = "javascript")]
    js_enabled: bool,
}

impl MultiScriptEngine {
    /// Create a new multi-script engine
    ///
    /// # Arguments
    /// * `js_enabled` - Whether to enable JavaScript support (requires plugin enabled)
    pub fn new(js_enabled: bool) -> Result<Self, QuicpulseError> {
        let rune_engine = ScriptEngine::new()?;

        #[cfg(feature = "javascript")]
        let js_engine = if js_enabled {
            Some(JsScriptEngine::new()?)
        } else {
            None
        };

        Ok(Self {
            rune_engine,
            #[cfg(feature = "javascript")]
            js_engine,
            #[cfg(feature = "javascript")]
            js_enabled,
        })
    }

    /// Create with only Rune support (JavaScript disabled)
    pub fn rune_only() -> Result<Self, QuicpulseError> {
        Self::new(false)
    }

    /// Check if JavaScript is enabled
    pub fn is_js_enabled(&self) -> bool {
        #[cfg(feature = "javascript")]
        {
            self.js_enabled && self.js_engine.is_some()
        }
        #[cfg(not(feature = "javascript"))]
        {
            false
        }
    }

    /// Execute a script with the appropriate engine based on script type
    pub async fn execute(
        &self,
        source: &str,
        ctx: &mut ScriptContext,
        script_type: ScriptType,
    ) -> Result<ScriptResult, QuicpulseError> {
        match script_type {
            ScriptType::Rune => {
                self.rune_engine.execute(source, ctx).await
            }
            ScriptType::JavaScript => {
                #[cfg(feature = "javascript")]
                {
                    match &self.js_engine {
                        Some(engine) => engine.execute(source, ctx).await,
                        None => Err(QuicpulseError::Config(
                            "JavaScript scripting is not enabled. Enable the 'quicpulse-javascript' plugin in your plugins config.".to_string()
                        )),
                    }
                }
                #[cfg(not(feature = "javascript"))]
                {
                    Err(QuicpulseError::Config(
                        "JavaScript support is not compiled into this build. Rebuild with the 'javascript' feature enabled.".to_string()
                    ))
                }
            }
        }
    }

    /// Compile a script (validates syntax without execution)
    pub fn compile(&self, source: &str, script_type: ScriptType) -> Result<(), QuicpulseError> {
        match script_type {
            ScriptType::Rune => {
                self.rune_engine.compile(source)?;
                Ok(())
            }
            ScriptType::JavaScript => {
                #[cfg(feature = "javascript")]
                {
                    match &self.js_engine {
                        Some(engine) => engine.compile(source),
                        None => Err(QuicpulseError::Config(
                            "JavaScript scripting is not enabled.".to_string()
                        )),
                    }
                }
                #[cfg(not(feature = "javascript"))]
                {
                    Err(QuicpulseError::Config(
                        "JavaScript support is not compiled into this build.".to_string()
                    ))
                }
            }
        }
    }

    /// Get the underlying Rune engine (for advanced usage)
    pub fn rune_engine(&self) -> &ScriptEngine {
        &self.rune_engine
    }

    /// Get the underlying JavaScript engine (if enabled)
    #[cfg(feature = "javascript")]
    pub fn js_engine(&self) -> Option<&JsScriptEngine> {
        self.js_engine.as_ref()
    }
}

impl Default for MultiScriptEngine {
    fn default() -> Self {
        Self::rune_only().expect("Failed to create default script engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rune_execution() {
        let engine = MultiScriptEngine::rune_only().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute(
            "pub fn main() { 1 + 1 }",
            &mut ctx,
            ScriptType::Rune
        ).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_js_disabled_by_default() {
        let engine = MultiScriptEngine::rune_only().unwrap();
        assert!(!engine.is_js_enabled());
    }

    #[cfg(feature = "javascript")]
    #[tokio::test]
    async fn test_js_when_enabled() {
        let engine = MultiScriptEngine::new(true).unwrap();
        assert!(engine.is_js_enabled());

        let mut ctx = ScriptContext::new();
        let result = engine.execute(
            "1 + 1",
            &mut ctx,
            ScriptType::JavaScript
        ).await;

        assert!(result.is_ok());
    }
}
