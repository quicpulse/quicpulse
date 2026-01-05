// Allow dead code for partially implemented features
#![allow(dead_code)]
#![allow(unused_imports)]

mod auth;
mod bench;
mod binary;
mod cli;
mod client;
mod config;
mod context;
mod cookies;
mod core;
mod devexp;
mod downloads;
mod encoding;
mod errors;
mod filter;
mod fs;
mod fuzz;
mod graphql;
mod grpc;
mod har;
mod http;
mod input;
mod internal;
mod json;
mod magic;
mod middleware;
mod mime;
mod mock;
mod models;
mod openapi;
mod output;
mod pipeline;
mod plugins;
mod request;
mod scripting;
mod sessions;
mod signals;
mod status;
mod strings;
mod table;
mod uploads;
mod utils;
mod websocket;

use context::Environment;
use status::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};

/// Entry point - catches Ctrl+C and calls core::run()
///
/// Returns ExitStatus directly, which implements std::process::Termination.
fn main() -> ExitStatus {
    // Set up Ctrl+C handler that sets a flag instead of calling exit()
    // This allows destructors to run and resources to be cleaned up properly
    ctrlc::set_handler(move || {
        // Set the flag using signals module
        signals::set_interrupted();

        // Print newline to clean up interrupted line
        eprintln!("\nInterrupted");

        // On second Ctrl+C, force exit (user really wants out)
        static SECOND_CTRL_C: AtomicBool = AtomicBool::new(false);
        if SECOND_CTRL_C.swap(true, Ordering::SeqCst) {
            // Second interrupt - force exit without cleanup
            std::process::exit(ExitStatus::Interrupted as i32);
        }
    })
    .ok();

    let args: Vec<String> = std::env::args().collect();
    let env = Environment::init();

    let status = core::run(args, env);

    if signals::was_interrupted() {
        return ExitStatus::Interrupted;
    }

    status
}
