//! QuicPulse library interface
//!
//! This crate provides a user-friendly HTTP client for the command line.
//!
//! # Module Organization
//!
//! - [`signals`] - Interrupt handling (was_interrupted, set_interrupted)
//! - [`errors`] - Error types (QuicpulseError, Result)
//! - [`status`] - Exit status codes (ExitStatus)
//! - [`core`] - Main execution logic

// Allow dead code for partially implemented features
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod auth;
pub mod bench;
pub mod binary;
pub mod cli;
pub mod client;
pub mod config;
pub mod context;
pub mod cookies;
pub mod core;
pub mod debug;
pub mod devexp;
pub mod downloads;
pub mod encoding;
pub mod errors;
pub mod filter;
pub mod fs;
pub mod fuzz;
pub mod graphql;
pub mod grpc;
pub mod har;
pub mod http;
pub mod input;
pub mod internal;
pub mod json;
pub mod k8s;
pub mod magic;
pub mod middleware;
pub mod mime;
pub mod mock;
pub mod models;
pub mod openapi;
pub mod output;
pub mod pipeline;
pub mod plugins;
pub mod request;
pub mod scripting;
pub mod sessions;
pub mod signals;
pub mod status;
pub mod strings;
pub mod table;
pub mod uploads;
pub mod utils;
pub mod websocket;
