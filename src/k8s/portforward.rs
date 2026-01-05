//! Kubernetes port-forward management
//!
//! Manages kubectl port-forward processes for transparent k8s:// URL handling.

use crate::errors::QuicpulseError;
use super::parser::K8sUrl;
use std::collections::HashMap;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A single port-forward connection
#[derive(Debug)]
pub struct PortForward {
    /// Local port being forwarded
    pub local_port: u16,
    /// Remote port on the service
    pub remote_port: u16,
    /// Service name
    pub service: String,
    /// Namespace
    pub namespace: String,
    /// kubectl process handle
    process: Child,
}

impl PortForward {
    /// Get the localhost URL for this port-forward
    pub fn local_url(&self, path: &str, query: Option<&str>) -> String {
        let mut url = format!("http://localhost:{}{}", self.local_port, path);
        if let Some(q) = query {
            url.push('?');
            url.push_str(q);
        }
        url
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        // Kill the kubectl process when the port-forward is dropped
        let _ = self.process.kill();
    }
}

/// Manages multiple port-forwards
#[derive(Debug, Clone)]
pub struct PortForwardManager {
    /// Active port-forwards, keyed by "service.namespace:port"
    forwards: Arc<Mutex<HashMap<String, Arc<PortForward>>>>,
}

impl Default for PortForwardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PortForwardManager {
    /// Create a new port-forward manager
    pub fn new() -> Self {
        Self {
            forwards: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a port-forward for a k8s URL
    ///
    /// If a port-forward already exists for this service/namespace/port combination,
    /// it will be reused. Otherwise, a new one will be created.
    pub async fn get_or_create(&self, k8s_url: &K8sUrl) -> Result<(u16, String), QuicpulseError> {
        let key = format!("{}.{}:{}", k8s_url.service, k8s_url.namespace, k8s_url.port);

        // Check if we already have a port-forward
        {
            let forwards = self.forwards.lock()
                .map_err(|e| QuicpulseError::Connection(format!("Lock error: {}", e)))?;

            if let Some(pf) = forwards.get(&key) {
                let url = pf.local_url(&k8s_url.path, k8s_url.query.as_deref());
                return Ok((pf.local_port, url));
            }
        }

        // Create a new port-forward
        let port_forward = create_port_forward(k8s_url)?;
        let local_port = port_forward.local_port;
        let local_url = port_forward.local_url(&k8s_url.path, k8s_url.query.as_deref());

        // Store it
        {
            let mut forwards = self.forwards.lock()
                .map_err(|e| QuicpulseError::Connection(format!("Lock error: {}", e)))?;
            forwards.insert(key, Arc::new(port_forward));
        }

        Ok((local_port, local_url))
    }

    /// Close all port-forwards
    pub fn close_all(&self) {
        if let Ok(mut forwards) = self.forwards.lock() {
            forwards.clear(); // Drop will kill all kubectl processes
        }
    }

    /// Get the number of active port-forwards
    pub fn active_count(&self) -> usize {
        self.forwards.lock().map(|f| f.len()).unwrap_or(0)
    }
}

impl Drop for PortForwardManager {
    fn drop(&mut self) {
        self.close_all();
    }
}

/// Create a new port-forward using kubectl
fn create_port_forward(k8s_url: &K8sUrl) -> Result<PortForward, QuicpulseError> {
    // Find an available local port
    let local_port = find_available_port()?;

    // Start kubectl port-forward
    let mut cmd = Command::new("kubectl");
    cmd.args([
        "port-forward",
        "-n", &k8s_url.namespace,
        &format!("svc/{}", k8s_url.service),
        &format!("{}:{}", local_port, k8s_url.port),
    ]);

    // Suppress output
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let process = cmd.spawn()
        .map_err(|e| QuicpulseError::Auth(format!(
            "Failed to start kubectl port-forward. Is kubectl installed and in PATH? Error: {}", e
        )))?;

    // Wait a moment for port-forward to establish
    std::thread::sleep(Duration::from_millis(500));

    // Verify the port is listening
    if !is_port_listening(local_port) {
        // Try to read stderr for error message
        return Err(QuicpulseError::Auth(format!(
            "kubectl port-forward failed to establish connection to {}/{} on port {}. \
             Verify the service exists and you have cluster access.",
            k8s_url.namespace, k8s_url.service, k8s_url.port
        )));
    }

    Ok(PortForward {
        local_port,
        remote_port: k8s_url.port,
        service: k8s_url.service.clone(),
        namespace: k8s_url.namespace.clone(),
        process,
    })
}

/// Find an available local port
fn find_available_port() -> Result<u16, QuicpulseError> {
    // Bind to port 0 to let the OS assign an available port
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| QuicpulseError::Connection(format!("Failed to find available port: {}", e)))?;

    let port = listener.local_addr()
        .map_err(|e| QuicpulseError::Connection(format!("Failed to get local address: {}", e)))?
        .port();

    // Drop the listener to release the port
    drop(listener);

    Ok(port)
}

/// Check if a port is listening (connection test)
fn is_port_listening(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

/// Global port-forward manager instance
static PORT_FORWARD_MANAGER: std::sync::OnceLock<PortForwardManager> = std::sync::OnceLock::new();

/// Get the global port-forward manager
pub fn get_port_forward_manager() -> &'static PortForwardManager {
    PORT_FORWARD_MANAGER.get_or_init(PortForwardManager::new)
}

/// Process a k8s:// URL and return the equivalent localhost URL
///
/// This is the main entry point for k8s URL handling.
pub async fn process_k8s_url(url: &str) -> Result<String, QuicpulseError> {
    let k8s_url = super::parser::parse_k8s_url(url)?;
    let manager = get_port_forward_manager();
    let (_, local_url) = manager.get_or_create(&k8s_url).await?;
    Ok(local_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_available_port() {
        let port = find_available_port().unwrap();
        assert!(port > 0);
        // Port should be in ephemeral range
        assert!(port > 1024);
    }

    #[test]
    fn test_port_forward_manager_new() {
        let manager = PortForwardManager::new();
        assert_eq!(manager.active_count(), 0);
    }
}
