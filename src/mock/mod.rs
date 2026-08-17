//! Built-in Mock Server
//!
//! Provides a simple mock HTTP server for testing, development, and debugging.
//! Supports static responses, dynamic templates, request logging, and recording.

pub mod config;
pub mod routes;
pub mod server;

pub use config::MockServerConfig;
pub use routes::{ResponseConfig, Route, RouteConfig};
pub use server::MockServer;
