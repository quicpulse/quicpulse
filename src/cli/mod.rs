//! CLI argument parsing and processing

pub mod args;
pub mod dicts;
pub mod process;

// Re-exports
pub use args::{Args, LogFormat};
pub use process::process_args;

// Backward compatibility alias
pub use process as parser;
