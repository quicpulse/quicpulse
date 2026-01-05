//! Built-in Mock Server
//!
//! Provides a simple mock HTTP server for testing, development, and debugging.
//! Supports static responses, dynamic templates, request logging, and recording.

pub mod server;
pub mod routes;
pub mod config;

pub use server::MockServer;
pub use routes::{Route, RouteConfig, ResponseConfig};
pub use config::MockServerConfig;
