//! Rune runtime and script engine
//!
//! This module provides the core Rune VM setup with custom modules
//! for HTTP operations, JSON manipulation, and utility functions.

use crate::errors::QuicpulseError;
use super::context::ScriptContext;
use rune::{
    Context, Diagnostics, Options, Source, Sources, Unit, Vm,
    termcolor::{ColorChoice, StandardStream},
};
use rune::runtime::{RuntimeContext, Value};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Result of script execution
#[derive(Debug, Clone)]
pub enum ScriptResult {
    /// No return value (unit)
    Unit,
    /// Boolean result
    Bool(bool),
    /// Integer result
    Integer(i64),
    /// Float result
    Float(f64),
    /// String result
    String(std::string::String),
    /// JSON object/array result
    Json(JsonValue),
    /// Raw Rune value (for complex types)
    Value(std::string::String),
}

impl ScriptResult {
    /// Convert to bool, error if not possible
    pub fn as_bool(&self) -> Result<bool, QuicpulseError> {
        match self {
            ScriptResult::Bool(b) => Ok(*b),
            ScriptResult::Integer(i) => Ok(*i != 0),
            ScriptResult::Unit => Ok(true),
            _ => Err(QuicpulseError::Script(format!(
                "Cannot convert {:?} to bool", self
            ))),
        }
    }

    /// Convert to JSON value
    pub fn as_json(&self) -> Result<JsonValue, QuicpulseError> {
        match self {
            ScriptResult::Unit => Ok(JsonValue::Null),
            ScriptResult::Bool(b) => Ok(JsonValue::Bool(*b)),
            ScriptResult::Integer(i) => Ok(JsonValue::Number((*i).into())),
            ScriptResult::Float(f) => Ok(serde_json::Number::from_f64(*f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)),
            ScriptResult::String(s) => Ok(JsonValue::String(s.clone())),
            ScriptResult::Json(j) => Ok(j.clone()),
            ScriptResult::Value(s) => serde_json::from_str(s)
                .map_err(|e| QuicpulseError::Script(format!("Invalid JSON: {}", e))),
        }
    }

    /// Convert to string representation
    pub fn as_string(&self) -> std::string::String {
        match self {
            ScriptResult::Unit => "()".to_string(),
            ScriptResult::Bool(b) => b.to_string(),
            ScriptResult::Integer(i) => i.to_string(),
            ScriptResult::Float(f) => f.to_string(),
            ScriptResult::String(s) => s.clone(),
            ScriptResult::Json(j) => serde_json::to_string(j).unwrap_or_default(),
            ScriptResult::Value(s) => s.clone(),
        }
    }
}

/// The main script engine powered by Rune
pub struct ScriptEngine {
    context: Context,
    runtime: Arc<RuntimeContext>,
}

impl ScriptEngine {
    /// Create a new script engine with all modules installed
    pub fn new() -> Result<Self, QuicpulseError> {
        let mut context = Context::with_default_modules()
            .map_err(|e| QuicpulseError::Script(format!("Failed to create context: {}", e)))?;

        // Install standard library modules
        Self::install_std_modules(&mut context)?;

        // Install custom QuicPulse modules
        Self::install_custom_modules(&mut context)?;

        let runtime = Arc::new(context.runtime()
            .map_err(|e| QuicpulseError::Script(format!("Failed to create runtime: {}", e)))?);

        Ok(Self { context, runtime })
    }

    /// Install standard Rune modules
    fn install_std_modules(context: &mut Context) -> Result<(), QuicpulseError> {
        // Install rune-modules if available (they take a bool parameter in 0.14)
        if let Ok(module) = rune_modules::json::module(true) {
            let _ = context.install(module);
        }

        if let Ok(module) = rune_modules::rand::module(true) {
            let _ = context.install(module);
        }

        if let Ok(module) = rune_modules::time::module(true) {
            let _ = context.install(module);
        }

        Ok(())
    }

    /// Install custom QuicPulse modules
    fn install_custom_modules(context: &mut Context) -> Result<(), QuicpulseError> {
        // Install our custom http module
        context.install(super::modules::http::module()
            .map_err(|e| QuicpulseError::Script(format!("http module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install http: {}", e)))?;

        // Install assert module
        context.install(super::modules::assert::module()
            .map_err(|e| QuicpulseError::Script(format!("assert module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install assert: {}", e)))?;

        // Install crypto module
        context.install(super::modules::crypto::module()
            .map_err(|e| QuicpulseError::Script(format!("crypto module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install crypto: {}", e)))?;

        // Install encoding module
        context.install(super::modules::encoding::module()
            .map_err(|e| QuicpulseError::Script(format!("encoding module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install encoding: {}", e)))?;

        // Install env module
        context.install(super::modules::env::module()
            .map_err(|e| QuicpulseError::Script(format!("env module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install env: {}", e)))?;

        // Install faker module for test data generation
        context.install(super::modules::faker::module()
            .map_err(|e| QuicpulseError::Script(format!("faker module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install faker: {}", e)))?;

        // Install prompt module for interactive input
        context.install(super::modules::prompt::module()
            .map_err(|e| QuicpulseError::Script(format!("prompt module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install prompt: {}", e)))?;

        // Install jwt module for token parsing
        context.install(super::modules::jwt::module()
            .map_err(|e| QuicpulseError::Script(format!("jwt module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install jwt: {}", e)))?;

        // Install fs module for sandboxed file access
        context.install(super::modules::fs::module()
            .map_err(|e| QuicpulseError::Script(format!("fs module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install fs: {}", e)))?;

        // Install store module for workflow state
        context.install(super::modules::store::module()
            .map_err(|e| QuicpulseError::Script(format!("store module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install store: {}", e)))?;

        // Install console module for structured logging
        context.install(super::modules::console::module()
            .map_err(|e| QuicpulseError::Script(format!("console module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install console: {}", e)))?;

        // Install system module for system utilities
        context.install(super::modules::system::module()
            .map_err(|e| QuicpulseError::Script(format!("system module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install system: {}", e)))?;

        // Install json module for JSON manipulation and JSONPath
        context.install(super::modules::json::module()
            .map_err(|e| QuicpulseError::Script(format!("json module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install json: {}", e)))?;

        // Install xml module for XML parsing
        context.install(super::modules::xml::module()
            .map_err(|e| QuicpulseError::Script(format!("xml module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install xml: {}", e)))?;

        // Install regex module for pattern matching
        context.install(super::modules::regex::module()
            .map_err(|e| QuicpulseError::Script(format!("regex module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install regex: {}", e)))?;

        // Install url module for URL manipulation
        context.install(super::modules::url::module()
            .map_err(|e| QuicpulseError::Script(format!("url module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install url: {}", e)))?;

        // Install date module for date/time operations
        context.install(super::modules::date::module()
            .map_err(|e| QuicpulseError::Script(format!("date module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install date: {}", e)))?;

        // Install cookie module for cookie handling
        context.install(super::modules::cookie::module()
            .map_err(|e| QuicpulseError::Script(format!("cookie module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install cookie: {}", e)))?;

        // Install schema module for JSON Schema validation
        context.install(super::modules::schema::module()
            .map_err(|e| QuicpulseError::Script(format!("schema module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install schema: {}", e)))?;

        // Install request module for HTTP requests from scripts
        context.install(super::modules::request::module()
            .map_err(|e| QuicpulseError::Script(format!("request module: {}", e)))?)
            .map_err(|e| QuicpulseError::Script(format!("Failed to install request: {}", e)))?;

        Ok(())
    }

    /// Compile a script without executing
    pub fn compile(&self, source: &str) -> Result<Arc<Unit>, QuicpulseError> {
        let mut sources = Sources::new();
        let _ = sources.insert(Source::memory(source)
            .map_err(|e| QuicpulseError::Script(format!("Source error: {}", e)))?);

        let mut diagnostics = Diagnostics::new();
        let options = Options::default();

        let result = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_options(&options)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            let _ = diagnostics.emit(&mut writer, &sources);
        }

        let unit = result.map_err(|e| QuicpulseError::Script(format!("Compile error: {}", e)))?;

        Ok(Arc::new(unit))
    }

    /// Execute a script with the given context
    pub async fn execute(&self, source: &str, _ctx: &mut ScriptContext) -> Result<ScriptResult, QuicpulseError> {
        let unit = self.compile(source)?;

        let vm = Vm::new(self.runtime.clone(), unit);

        // Call the main function
        let execution = vm.send_execute(rune::Hash::type_hash(["main"]), ())
            .map_err(|e| QuicpulseError::Script(format!("Execution setup error: {}", e)))?;

        let output: Value = execution.async_complete().await
            .into_result()
            .map_err(|e| QuicpulseError::Script(format!("Execution error: {}", e)))?;

        // Convert the output to ScriptResult
        Self::value_to_result(output)
    }

    /// Convert a Rune Value to ScriptResult
    fn value_to_result(value: Value) -> Result<ScriptResult, QuicpulseError> {
        // Try to convert based on what the value is
        // First, try basic types using from_value
        if let Ok(b) = rune::from_value::<bool>(value.clone()) {
            return Ok(ScriptResult::Bool(b));
        }

        if let Ok(i) = rune::from_value::<i64>(value.clone()) {
            return Ok(ScriptResult::Integer(i));
        }

        if let Ok(f) = rune::from_value::<f64>(value.clone()) {
            return Ok(ScriptResult::Float(f));
        }

        if let Ok(s) = rune::from_value::<std::string::String>(value.clone()) {
            return Ok(ScriptResult::String(s));
        }

        if let Ok(()) = rune::from_value::<()>(value.clone()) {
            return Ok(ScriptResult::Unit);
        }

        // For complex types, try to format them
        Ok(ScriptResult::Value(format!("{:?}", value)))
    }
}

impl Clone for ScriptEngine {
    fn clone(&self) -> Self {
        // Create a new engine with the same configuration
        Self::new().expect("Failed to clone ScriptEngine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_execution() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute("pub fn main() { 42 }", &mut ctx).await;
        assert!(result.is_ok());

        if let Ok(ScriptResult::Integer(n)) = result {
            assert_eq!(n, 42);
        }
    }

    #[tokio::test]
    async fn test_bool_result() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute("pub fn main() { true }", &mut ctx).await;
        assert!(result.is_ok());

        if let Ok(ScriptResult::Bool(b)) = result {
            assert!(b);
        }
    }

    #[tokio::test]
    async fn test_string_result() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute(r#"pub fn main() { "hello" }"#, &mut ctx).await;
        assert!(result.is_ok());
    }
}
