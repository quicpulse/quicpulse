//! Plugin hooks system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hook types available for plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHook {
    /// Called before parsing CLI arguments
    PreParse,
    /// Called after parsing CLI arguments
    PostParse,
    /// Called before building the request
    PreRequest,
    /// Called after building the request, before sending
    PostRequest,
    /// Called after receiving a response
    PreResponse,
    /// Called before formatting output
    PostResponse,
    /// Called before output is printed
    PreOutput,
    /// Called after output is printed
    PostOutput,
    /// Called on errors
    OnError,
    /// Custom authentication hook
    Auth,
    /// Custom output formatter
    Format,
}

impl PluginHook {
    /// Get hook name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginHook::PreParse => "pre_parse",
            PluginHook::PostParse => "post_parse",
            PluginHook::PreRequest => "pre_request",
            PluginHook::PostRequest => "post_request",
            PluginHook::PreResponse => "pre_response",
            PluginHook::PostResponse => "post_response",
            PluginHook::PreOutput => "pre_output",
            PluginHook::PostOutput => "post_output",
            PluginHook::OnError => "on_error",
            PluginHook::Auth => "auth",
            PluginHook::Format => "format",
        }
    }

    /// Parse hook name from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pre_parse" => Some(PluginHook::PreParse),
            "post_parse" => Some(PluginHook::PostParse),
            "pre_request" => Some(PluginHook::PreRequest),
            "post_request" => Some(PluginHook::PostRequest),
            "pre_response" => Some(PluginHook::PreResponse),
            "post_response" => Some(PluginHook::PostResponse),
            "pre_output" => Some(PluginHook::PreOutput),
            "post_output" => Some(PluginHook::PostOutput),
            "on_error" => Some(PluginHook::OnError),
            "auth" => Some(PluginHook::Auth),
            "format" => Some(PluginHook::Format),
            _ => None,
        }
    }

    /// Get all available hooks
    pub fn all() -> Vec<Self> {
        vec![
            PluginHook::PreParse,
            PluginHook::PostParse,
            PluginHook::PreRequest,
            PluginHook::PostRequest,
            PluginHook::PreResponse,
            PluginHook::PostResponse,
            PluginHook::PreOutput,
            PluginHook::PostOutput,
            PluginHook::OnError,
            PluginHook::Auth,
            PluginHook::Format,
        ]
    }
}

/// Context passed to hook handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// Hook type being executed
    pub hook: String,

    /// Request URL (if applicable)
    pub url: Option<String>,

    /// HTTP method (if applicable)
    pub method: Option<String>,

    /// Request headers (if applicable)
    pub request_headers: HashMap<String, String>,

    /// Request body (if applicable)
    pub request_body: Option<String>,

    /// Response status (if applicable)
    pub response_status: Option<u16>,

    /// Response headers (if applicable)
    pub response_headers: HashMap<String, String>,

    /// Response body (if applicable)
    pub response_body: Option<String>,

    /// Error message (if applicable)
    pub error: Option<String>,

    /// Plugin-specific data
    pub data: HashMap<String, serde_json::Value>,
}

impl HookContext {
    /// Create a new empty context
    pub fn new(hook: PluginHook) -> Self {
        Self {
            hook: hook.as_str().to_string(),
            url: None,
            method: None,
            request_headers: HashMap::new(),
            request_body: None,
            response_status: None,
            response_headers: HashMap::new(),
            response_body: None,
            error: None,
            data: HashMap::new(),
        }
    }

    /// Create context for pre-request hook
    pub fn pre_request(url: &str, method: &str, headers: HashMap<String, String>, body: Option<String>) -> Self {
        Self {
            hook: PluginHook::PreRequest.as_str().to_string(),
            url: Some(url.to_string()),
            method: Some(method.to_string()),
            request_headers: headers,
            request_body: body,
            ..Self::new(PluginHook::PreRequest)
        }
    }

    /// Create context for post-response hook
    pub fn post_response(
        url: &str,
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    ) -> Self {
        Self {
            hook: PluginHook::PostResponse.as_str().to_string(),
            url: Some(url.to_string()),
            response_status: Some(status),
            response_headers: headers,
            response_body: Some(body),
            ..Self::new(PluginHook::PostResponse)
        }
    }

    /// Create context for error hook
    pub fn on_error(error: &str) -> Self {
        Self {
            hook: PluginHook::OnError.as_str().to_string(),
            error: Some(error.to_string()),
            ..Self::new(PluginHook::OnError)
        }
    }
}

/// Result from a hook handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Whether to continue processing
    pub continue_processing: bool,

    /// Modified URL (for pre-request hooks)
    pub url: Option<String>,

    /// Modified method (for pre-request hooks)
    pub method: Option<String>,

    /// Modified/additional headers
    pub headers: HashMap<String, String>,

    /// Headers to remove
    pub remove_headers: Vec<String>,

    /// Modified body
    pub body: Option<String>,

    /// Output to print (for output hooks)
    pub output: Option<String>,

    /// Error message (to abort with error)
    pub error: Option<String>,

    /// Additional data to pass to next hooks
    pub data: HashMap<String, serde_json::Value>,
}

impl Default for HookResult {
    fn default() -> Self {
        Self {
            continue_processing: true,
            url: None,
            method: None,
            headers: HashMap::new(),
            remove_headers: Vec::new(),
            body: None,
            output: None,
            error: None,
            data: HashMap::new(),
        }
    }
}

impl HookResult {
    /// Create a result that continues processing
    pub fn ok() -> Self {
        Self::default()
    }

    /// Create a result that stops processing
    pub fn stop() -> Self {
        Self {
            continue_processing: false,
            ..Default::default()
        }
    }

    /// Create a result with an error
    pub fn error(msg: &str) -> Self {
        Self {
            continue_processing: false,
            error: Some(msg.to_string()),
            ..Default::default()
        }
    }

    /// Create a result with modified URL
    pub fn with_url(url: &str) -> Self {
        Self {
            url: Some(url.to_string()),
            ..Default::default()
        }
    }

    /// Create a result with additional headers
    pub fn with_headers(headers: HashMap<String, String>) -> Self {
        Self {
            headers,
            ..Default::default()
        }
    }

    /// Create a result with modified body
    pub fn with_body(body: String) -> Self {
        Self {
            body: Some(body),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_names() {
        assert_eq!(PluginHook::PreRequest.as_str(), "pre_request");
        assert_eq!(PluginHook::from_str("post_response"), Some(PluginHook::PostResponse));
    }

    #[test]
    fn test_hook_context() {
        let ctx = HookContext::pre_request(
            "http://example.com",
            "GET",
            HashMap::new(),
            None,
        );
        assert_eq!(ctx.url, Some("http://example.com".to_string()));
        assert_eq!(ctx.method, Some("GET".to_string()));
    }

    #[test]
    fn test_hook_result() {
        let result = HookResult::ok();
        assert!(result.continue_processing);

        let result = HookResult::error("test error");
        assert!(!result.continue_processing);
        assert_eq!(result.error, Some("test error".to_string()));
    }
}
