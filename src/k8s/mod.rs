//! Kubernetes integration module
//!
//! Provides transparent port-forwarding for `k8s://` URLs, allowing direct
//! requests to Kubernetes services without manual kubectl port-forward setup.
//!
//! # URL Format
//! ```text
//! k8s://service.namespace[:port][/path]
//! ```
//!
//! # Examples
//! ```text
//! k8s://api-server.default:8080/health
//! k8s://my-service.production/api/v1/users
//! k8s://grafana.monitoring:3000
//! ```

pub mod parser;
pub mod portforward;

pub use parser::{parse_k8s_url, K8sUrl};
pub use portforward::{PortForward, PortForwardManager};
