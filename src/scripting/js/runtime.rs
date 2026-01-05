//! JavaScript runtime using QuickJS via rquickjs
//!
//! Provides the JsScriptEngine that executes JavaScript code with access
//! to the same modules available in Rune scripts.

use rquickjs::{Context, Runtime, Function, Value, Ctx};
use std::sync::Arc;

use crate::errors::QuicpulseError;
use crate::scripting::context::ScriptContext;
use crate::scripting::runtime::ScriptResult;
use super::context::inject_context;
use super::modules;

/// JavaScript script engine powered by QuickJS
pub struct JsScriptEngine {
    runtime: Runtime,
}

impl JsScriptEngine {
    /// Create a new JavaScript engine
    pub fn new() -> Result<Self, QuicpulseError> {
        let runtime = Runtime::new()
            .map_err(|e| QuicpulseError::Script(format!("Failed to create JS runtime: {}", e)))?;

        // Set memory limit (64MB) to prevent runaway scripts
        runtime.set_memory_limit(64 * 1024 * 1024);

        // Set max stack size
        runtime.set_max_stack_size(1024 * 1024);

        Ok(Self { runtime })
    }

    /// Execute JavaScript code and return the result
    pub async fn execute(
        &self,
        source: &str,
        ctx: &mut ScriptContext,
    ) -> Result<ScriptResult, QuicpulseError> {
        let context = Context::full(&self.runtime)
            .map_err(|e| QuicpulseError::Script(format!("Failed to create JS context: {}", e)))?;

        // Clone data we need for the closure
        let source = source.to_string();
        let request = ctx.get_request();
        let response = ctx.get_response();
        let variables = ctx.variables().clone();
        let env = ctx.env().clone();

        let result = context.with(|ctx| {
            // Register all modules
            modules::register_all(&ctx)?;

            // Inject script context (request, response, variables)
            inject_context(&ctx, request.as_ref(), response.as_ref(), &variables, &env)?;

            // Evaluate the script
            let result: Value = ctx.eval(source.as_bytes())
                .map_err(|e| QuicpulseError::Script(format!("JS execution error: {}", e)))?;

            // Convert result to ScriptResult
            convert_js_value(&ctx, result)
        })?;

        Ok(result)
    }

    /// Compile/validate JavaScript code without executing
    pub fn compile(&self, source: &str) -> Result<(), QuicpulseError> {
        let context = Context::full(&self.runtime)
            .map_err(|e| QuicpulseError::Script(format!("Failed to create JS context: {}", e)))?;

        context.with(|ctx| {
            // Try to compile as a function to validate syntax
            let wrapped = format!("(function() {{ {} }})", source);
            ctx.eval::<Value, _>(wrapped.as_bytes())
                .map_err(|e| QuicpulseError::Script(format!("JS syntax error: {}", e)))?;
            Ok(())
        })
    }
}

/// Convert a QuickJS Value to ScriptResult
fn convert_js_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<ScriptResult, QuicpulseError> {
    if value.is_undefined() || value.is_null() {
        return Ok(ScriptResult::Unit);
    }

    if let Some(b) = value.as_bool() {
        return Ok(ScriptResult::Bool(b));
    }

    if let Some(n) = value.as_int() {
        return Ok(ScriptResult::Integer(n as i64));
    }

    if let Some(n) = value.as_float() {
        return Ok(ScriptResult::Float(n));
    }

    if let Some(s) = value.as_string() {
        let s = s.to_string()
            .map_err(|e| QuicpulseError::Script(format!("Failed to convert string: {}", e)))?;
        return Ok(ScriptResult::String(s));
    }

    // Try to convert objects/arrays to JSON
    if value.is_object() || value.is_array() {
        let json_str = ctx.globals()
            .get::<_, Function>("JSON")
            .and_then(|json| json.get::<_, Function>("stringify"))
            .and_then(|stringify| stringify.call::<_, String>((value.clone(),)));

        match json_str {
            Ok(s) => {
                let json: serde_json::Value = serde_json::from_str(&s)
                    .map_err(|e| QuicpulseError::Script(format!("Failed to parse JSON: {}", e)))?;
                return Ok(ScriptResult::Json(json));
            }
            Err(_) => {
                // Fallback to string representation
                return Ok(ScriptResult::Value(format!("{:?}", value)));
            }
        }
    }

    // Fallback
    Ok(ScriptResult::Value(format!("{:?}", value)))
}

impl Default for JsScriptEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default JS engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_expression() {
        let engine = JsScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute("1 + 1", &mut ctx).await.unwrap();
        assert_eq!(result.as_bool().ok(), Some(true)); // 2 is truthy
    }

    #[tokio::test]
    async fn test_boolean_result() {
        let engine = JsScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute("true", &mut ctx).await.unwrap();
        assert!(matches!(result, ScriptResult::Bool(true)));
    }

    #[tokio::test]
    async fn test_string_result() {
        let engine = JsScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let result = engine.execute("'hello'", &mut ctx).await.unwrap();
        if let ScriptResult::String(s) = result {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected string result");
        }
    }

    #[test]
    fn test_compile_valid() {
        let engine = JsScriptEngine::new().unwrap();
        assert!(engine.compile("const x = 1 + 1;").is_ok());
    }

    #[test]
    fn test_compile_invalid() {
        let engine = JsScriptEngine::new().unwrap();
        assert!(engine.compile("const x = ").is_err());
    }
}
