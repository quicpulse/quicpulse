//! Mock server configuration

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::errors::QuicpulseError;
use super::routes::RouteConfig;

/// Mock server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Enable request logging
    #[serde(default = "default_true")]
    pub log_requests: bool,

    /// Enable CORS headers
    #[serde(default)]
    pub cors: bool,

    /// Record requests to file
    #[serde(default)]
    pub record_to: Option<String>,

    /// Default response for unmatched routes
    #[serde(default)]
    pub default_response: Option<super::routes::ResponseConfig>,

    /// Routes
    #[serde(default)]
    pub routes: Vec<RouteConfig>,

    /// Enable SSL/TLS
    #[serde(default)]
    pub tls: Option<TlsConfig>,

    /// Proxy mode - forward unmatched requests to another server
    #[serde(default)]
    pub proxy_to: Option<String>,

    /// Latency simulation (min, max) in milliseconds
    #[serde(default)]
    pub latency: Option<(u64, u64)>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_true() -> bool {
    true
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert: String,
    /// Path to private key file
    pub key: String,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_requests: true,
            cors: false,
            record_to: None,
            default_response: None,
            routes: Vec::new(),
            tls: None,
            proxy_to: None,
            latency: None,
        }
    }
}

impl MockServerConfig {
    /// Create a new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Add a route
    pub fn add_route(mut self, route: RouteConfig) -> Self {
        self.routes.push(route);
        self
    }

    /// Load config from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, QuicpulseError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(QuicpulseError::Io)?;

        let ext = path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        match ext {
            "yaml" | "yml" => {
                serde_yaml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse YAML config: {}", e)))
            }
            "toml" => {
                toml::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse TOML config: {}", e)))
            }
            _ => {
                serde_json::from_str(&content)
                    .map_err(|e| QuicpulseError::Config(format!("Failed to parse JSON config: {}", e)))
            }
        }
    }

    /// Load config from YAML string
    pub fn from_yaml(content: &str) -> Result<Self, QuicpulseError> {
        serde_yaml::from_str(content)
            .map_err(|e| QuicpulseError::Config(format!("Failed to parse YAML: {}", e)))
    }

    /// Get address string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), QuicpulseError> {
        // Check routes compile
        for route in &self.routes {
            super::routes::Route::new(route.clone())
                .map_err(|e| QuicpulseError::Config(format!("Invalid route '{}': {}", route.path, e)))?;
        }

        // Check TLS files exist
        if let Some(ref tls) = self.tls {
            if !Path::new(&tls.cert).exists() {
                return Err(QuicpulseError::Config(format!("TLS cert file not found: {}", tls.cert)));
            }
            if !Path::new(&tls.key).exists() {
                return Err(QuicpulseError::Config(format!("TLS key file not found: {}", tls.key)));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MockServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.log_requests);
    }

    #[test]
    fn test_yaml_config() {
        let yaml = r#"
host: 0.0.0.0
port: 9000
cors: true
routes:
  - method: GET
    path: /api/health
    response:
      status: 200
      body: "OK"
  - method: POST
    path: /api/users
    response:
      status: 201
      json:
        id: 1
        name: "Created"
"#;
        let config = MockServerConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.port, 9000);
        assert!(config.cors);
        assert_eq!(config.routes.len(), 2);
    }
}
