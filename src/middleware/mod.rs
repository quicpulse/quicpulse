//! Tower-style middleware for HTTP request processing
//!
//! This module provides composable middleware using idiomatic Rust patterns
//! rather than Python-style trait object inheritance.

pub mod auth;

pub use auth::{Auth, AuthError};
