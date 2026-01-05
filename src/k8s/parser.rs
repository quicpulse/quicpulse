//! Kubernetes URL parsing
//!
//! Parses `k8s://` URLs into components for port-forwarding.

use crate::errors::QuicpulseError;

/// Parsed Kubernetes URL components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct K8sUrl {
    /// Service name
    pub service: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Target port on the service (default: 80)
    pub port: u16,
    /// Path component of the URL
    pub path: String,
    /// Query string (if any)
    pub query: Option<String>,
}

impl K8sUrl {
    /// Convert to a localhost URL with the given local port
    pub fn to_local_url(&self, local_port: u16) -> String {
        let mut url = format!("http://localhost:{}{}", local_port, self.path);
        if let Some(ref query) = self.query {
            url.push('?');
            url.push_str(query);
        }
        url
    }

    /// Get the kubectl port-forward target
    pub fn port_forward_target(&self) -> String {
        format!("svc/{}", self.service)
    }
}

/// Parse a k8s:// URL into components
///
/// # URL Format
/// ```text
/// k8s://service.namespace[:port][/path][?query]
/// ```
///
/// # Examples
/// ```
/// use quicpulse::k8s::parse_k8s_url;
///
/// let url = parse_k8s_url("k8s://api-server.default:8080/health").unwrap();
/// assert_eq!(url.service, "api-server");
/// assert_eq!(url.namespace, "default");
/// assert_eq!(url.port, 8080);
/// assert_eq!(url.path, "/health");
/// ```
pub fn parse_k8s_url(url: &str) -> Result<K8sUrl, QuicpulseError> {
    // Must start with k8s://
    let url = url.strip_prefix("k8s://")
        .ok_or_else(|| QuicpulseError::Argument(
            "K8s URL must start with k8s://".to_string()
        ))?;

    // Split path and query from host
    let (host_port, path_query) = match url.find('/') {
        Some(idx) => (&url[..idx], &url[idx..]),
        None => (url, "/"),
    };

    // Split path and query
    let (path, query) = match path_query.find('?') {
        Some(idx) => (&path_query[..idx], Some(path_query[idx + 1..].to_string())),
        None => (path_query, None),
    };

    // Parse host:port
    let (host, port) = match host_port.rfind(':') {
        Some(idx) => {
            let port_str = &host_port[idx + 1..];
            let port: u16 = port_str.parse()
                .map_err(|_| QuicpulseError::Argument(format!(
                    "Invalid port in k8s URL: {}", port_str
                )))?;
            (&host_port[..idx], port)
        }
        None => (host_port, 80),
    };

    // Parse service.namespace
    let (service, namespace) = match host.find('.') {
        Some(idx) => (&host[..idx], &host[idx + 1..]),
        None => {
            return Err(QuicpulseError::Argument(format!(
                "K8s URL must include namespace: k8s://service.namespace. Got: {}",
                host
            )));
        }
    };

    // Validate components
    if service.is_empty() {
        return Err(QuicpulseError::Argument(
            "Service name cannot be empty in k8s URL".to_string()
        ));
    }

    if namespace.is_empty() {
        return Err(QuicpulseError::Argument(
            "Namespace cannot be empty in k8s URL".to_string()
        ));
    }

    // Validate service and namespace names (Kubernetes DNS naming rules)
    validate_k8s_name(service, "service")?;
    validate_k8s_name(namespace, "namespace")?;

    Ok(K8sUrl {
        service: service.to_string(),
        namespace: namespace.to_string(),
        port,
        path: path.to_string(),
        query,
    })
}

/// Validate a Kubernetes resource name
fn validate_k8s_name(name: &str, kind: &str) -> Result<(), QuicpulseError> {
    // K8s names must:
    // - Be 63 characters or fewer
    // - Contain only lowercase alphanumeric characters or '-'
    // - Start with an alphabetic character
    // - End with an alphanumeric character

    if name.len() > 63 {
        return Err(QuicpulseError::Argument(format!(
            "K8s {} name too long (max 63 chars): {}", kind, name
        )));
    }

    if !name.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
        return Err(QuicpulseError::Argument(format!(
            "K8s {} name must start with a lowercase letter: {}", kind, name
        )));
    }

    if !name.chars().last().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) {
        return Err(QuicpulseError::Argument(format!(
            "K8s {} name must end with alphanumeric character: {}", kind, name
        )));
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(QuicpulseError::Argument(format!(
                "K8s {} name contains invalid character '{}': {}", kind, c, name
            )));
        }
    }

    Ok(())
}

/// Check if a URL is a k8s:// URL
pub fn is_k8s_url(url: &str) -> bool {
    url.starts_with("k8s://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_url() {
        let url = parse_k8s_url("k8s://api-server.default").unwrap();
        assert_eq!(url.service, "api-server");
        assert_eq!(url.namespace, "default");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/");
        assert_eq!(url.query, None);
    }

    #[test]
    fn test_parse_url_with_port() {
        let url = parse_k8s_url("k8s://grafana.monitoring:3000").unwrap();
        assert_eq!(url.service, "grafana");
        assert_eq!(url.namespace, "monitoring");
        assert_eq!(url.port, 3000);
    }

    #[test]
    fn test_parse_url_with_path() {
        let url = parse_k8s_url("k8s://api.prod:8080/api/v1/users").unwrap();
        assert_eq!(url.path, "/api/v1/users");
    }

    #[test]
    fn test_parse_url_with_query() {
        let url = parse_k8s_url("k8s://api.prod/search?q=foo&limit=10").unwrap();
        assert_eq!(url.path, "/search");
        assert_eq!(url.query, Some("q=foo&limit=10".to_string()));
    }

    #[test]
    fn test_to_local_url() {
        let url = parse_k8s_url("k8s://api.prod:8080/health?verbose=true").unwrap();
        assert_eq!(url.to_local_url(12345), "http://localhost:12345/health?verbose=true");
    }

    #[test]
    fn test_missing_namespace() {
        let result = parse_k8s_url("k8s://just-service");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_port() {
        let result = parse_k8s_url("k8s://api.prod:notaport");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_service_name() {
        let result = parse_k8s_url("k8s://Invalid.default");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_k8s_url() {
        assert!(is_k8s_url("k8s://api.default"));
        assert!(!is_k8s_url("http://api.default"));
        assert!(!is_k8s_url("https://api.default"));
    }
}
