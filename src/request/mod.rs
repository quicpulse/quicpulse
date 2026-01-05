//! Request building module
//!
//! Provides types and utilities for building HTTP requests from parsed input items.

mod builder;
mod json;

pub use builder::{RequestConfig, RequestBody, FileField};
pub use json::set_nested_value;
