//! Scripting support for QuicPulse
//!
//! This module provides embedded scripting capabilities using Rune and JavaScript,
//! enabling dynamic request/response transformations, custom assertions,
//! pre/post request hooks, and workflow automation.
//!
//! # Features
//!
//! - **Request transformation**: Modify requests before sending
//! - **Response transformation**: Transform responses before processing
//! - **Custom assertions**: Write complex assertion logic in scripts
//! - **Pre/post hooks**: Execute code before/after requests
//! - **Workflow scripting**: Full scripting support in workflows
//! - **Built-in modules**: HTTP, JSON, crypto, encoding utilities
//! - **Multi-language support**: Rune (default) and JavaScript via QuickJS

pub mod runtime;
pub mod context;
pub mod modules;
pub mod detection;
pub mod engine;

#[cfg(feature = "javascript")]
pub mod js;

pub use runtime::{ScriptEngine, ScriptResult};
pub use context::{ScriptContext, RequestData, ResponseData};
pub use detection::{ScriptType, detect_script_type};
pub use engine::MultiScriptEngine;

use crate::errors::QuicpulseError;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::sync::Arc;

/// Script execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptMode {
    /// Transform a request before sending
    PreRequest,
    /// Transform a response after receiving
    PostResponse,
    /// Evaluate an assertion (returns bool)
    Assertion,
    /// Extract a value from response
    Extract,
    /// General purpose script
    General,
}

/// A compiled script ready for execution
#[derive(Clone)]
pub struct CompiledScript {
    engine: Arc<ScriptEngine>,
    source: String,
    mode: ScriptMode,
}

impl CompiledScript {
    /// Create a new compiled script
    pub fn new(source: &str, mode: ScriptMode) -> Result<Self, QuicpulseError> {
        let engine = ScriptEngine::new()?;
        engine.compile(source)?;

        Ok(Self {
            engine: Arc::new(engine),
            source: source.to_string(),
            mode,
        })
    }

    /// Load and compile a script from a file
    pub fn from_file(path: &Path, mode: ScriptMode) -> Result<Self, QuicpulseError> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| QuicpulseError::Io(e))?;
        Self::new(&source, mode)
    }

    /// Execute the script with the given context
    pub async fn execute(&self, ctx: &mut ScriptContext) -> Result<ScriptResult, QuicpulseError> {
        self.engine.execute(&self.source, ctx).await
    }

    /// Execute as a pre-request hook, returning modified request data
    pub async fn execute_pre_request(&self, request: &mut RequestData) -> Result<(), QuicpulseError> {
        let mut ctx = ScriptContext::new();
        ctx.set_request(request.clone());

        let _result = self.execute(&mut ctx).await?;

        // Update request from context
        if let Some(modified) = ctx.get_request() {
            *request = modified;
        }

        Ok(())
    }

    /// Execute as a post-response hook
    pub async fn execute_post_response(&self, response: &mut ResponseData) -> Result<(), QuicpulseError> {
        let mut ctx = ScriptContext::new();
        ctx.set_response(response.clone());

        let _result = self.execute(&mut ctx).await?;

        // Update response from context
        if let Some(modified) = ctx.get_response() {
            *response = modified;
        }

        Ok(())
    }

    /// Execute as an assertion, returning true/false
    pub async fn execute_assertion(&self, ctx: &mut ScriptContext) -> Result<bool, QuicpulseError> {
        let result = self.execute(ctx).await?;
        result.as_bool()
    }

    /// Execute to extract a value
    pub async fn execute_extract(&self, ctx: &mut ScriptContext) -> Result<JsonValue, QuicpulseError> {
        let result = self.execute(ctx).await?;
        result.as_json()
    }

    /// Get the script source
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the script mode
    pub fn mode(&self) -> ScriptMode {
        self.mode
    }
}

/// Quick script execution without pre-compilation
pub async fn run_script(source: &str, ctx: &mut ScriptContext) -> Result<ScriptResult, QuicpulseError> {
    let engine = ScriptEngine::new()?;
    engine.execute(source, ctx).await
}

/// Run an inline assertion script
pub async fn run_assertion(source: &str, response: &ResponseData) -> Result<bool, QuicpulseError> {
    let mut ctx = ScriptContext::new();
    ctx.set_response(response.clone());

    let engine = ScriptEngine::new()?;
    let result = engine.execute(source, &mut ctx).await?;
    result.as_bool()
}

/// Transform request with a script
pub async fn transform_request(source: &str, request: &mut RequestData) -> Result<(), QuicpulseError> {
    let mut ctx = ScriptContext::new();
    ctx.set_request(request.clone());

    let engine = ScriptEngine::new()?;
    engine.execute(source, &mut ctx).await?;

    if let Some(modified) = ctx.get_request() {
        *request = modified;
    }

    Ok(())
}

/// Transform response with a script
pub async fn transform_response(source: &str, response: &mut ResponseData) -> Result<(), QuicpulseError> {
    let mut ctx = ScriptContext::new();
    ctx.set_response(response.clone());

    let engine = ScriptEngine::new()?;
    engine.execute(source, &mut ctx).await?;

    if let Some(modified) = ctx.get_response() {
        *response = modified;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_script() {
        let mut ctx = ScriptContext::new();
        ctx.set_variable("x", JsonValue::Number(42.into()));

        let result = run_script("pub fn main() { 1 + 1 }", &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assertion_script() {
        let response = ResponseData {
            status: 200,
            headers: Default::default(),
            body: serde_json::json!({"success": true}),
            elapsed_ms: 100,
        };

        // Simple assertion that returns true
        let result = run_assertion(
            r#"pub fn main() { true }"#,
            &response
        ).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
