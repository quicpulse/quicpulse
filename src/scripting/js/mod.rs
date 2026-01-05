//! JavaScript scripting support via QuickJS (rquickjs)
//!
//! This module provides JavaScript scripting capabilities as an optional plugin.
//! It is disabled by default and must be explicitly enabled.

mod runtime;
mod context;
pub mod modules;

pub use runtime::JsScriptEngine;
pub use context::inject_context;
