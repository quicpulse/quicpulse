//! Custom Rune modules for QuicPulse
//!
//! These modules provide comprehensive scripting capabilities:
//!
//! - **http**: HTTP constants and helpers
//! - **assert**: Test assertions and expectations
//! - **crypto**: Cryptographic operations (hashing, HMAC, UUIDs)
//! - **encoding**: Encoding utilities (Base64, hex, URL encoding)
//! - **env**: Environment variable access
//! - **faker**: Fake data generation for testing
//! - **prompt**: Interactive user input
//! - **jwt**: JWT token parsing and debugging
//! - **fs**: Sandboxed file system access
//! - **store**: Global key-value store for workflows
//! - **console**: Structured logging to stderr
//! - **system**: System utilities (sleep, timing, system info)
//! - **json**: JSON manipulation and JSONPath queries
//! - **xml**: XML parsing and conversion
//! - **regex**: Regular expression operations
//! - **url**: URL parsing and manipulation
//! - **date**: Date/time operations
//! - **cookie**: Cookie parsing and building
//! - **schema**: JSON Schema validation
//! - **request**: HTTP request invocation from scripts

pub mod http;
pub mod assert;
pub mod crypto;
pub mod encoding;
pub mod env;
pub mod faker;
pub mod prompt;
pub mod jwt;
pub mod fs;
pub mod store;
pub mod console;
pub mod system;
pub mod json;
pub mod xml;
pub mod regex;
pub mod url;
pub mod date;
pub mod cookie;
pub mod schema;
pub mod request;
