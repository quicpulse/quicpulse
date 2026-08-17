//! JavaScript scripting support via QuickJS (rquickjs)
//!
//! This module provides JavaScript scripting capabilities as an optional plugin.
//! It is disabled by default and must be explicitly enabled.

mod context;
pub mod modules;
mod runtime;

pub use context::inject_context;
pub use runtime::JsScriptEngine;
