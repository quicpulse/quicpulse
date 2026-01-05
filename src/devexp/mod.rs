//! Developer Experience features
//!
//! This module provides tools to improve developer workflow:
//!
//! - **Curl Generation**: Convert QuicPulse commands to equivalent curl commands
//! - **Curl Import**: Parse and replay curl commands
//! - **Environment Variables**: Load .env files and expand {{variable}} syntax
//!
//! # Curl Generation
//!
//! ```bash
//! # Generate curl command instead of sending request
//! quicpulse --curl POST api.example.com/users name=John
//!
//! # Output:
//! # curl -X POST -H 'Content-Type: application/json' -d '{"name":"John"}' 'https://api.example.com/users'
//! ```
//!
//! # Curl Import
//!
//! ```bash
//! # Import and execute a curl command
//! quicpulse --import-curl "curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"John\"}' https://api.example.com/users"
//! ```
//!
//! # Environment Variables
//!
//! Create a `.env` file:
//! ```text
//! API_KEY=secret123
//! BASE_URL=https://api.example.com
//! ```
//!
//! Use variables in requests:
//! ```bash
//! quicpulse {{BASE_URL}}/users Authorization:"Bearer {{API_KEY}}"
//! ```

pub mod codegen;
pub mod curl;
pub mod curl_import;
pub mod dotenv;
pub mod http_file;

pub use codegen::generate_code;
pub use curl::{generate_curl_command, format_curl_pretty};
pub use curl_import::{import_curl, parse_curl_command, ParsedCurl};
pub use dotenv::{EnvVars, has_variables};
pub use http_file::{parse_http_file, parse_http_content, HttpRequest, request_to_args, list_requests};
