//! HTTP client functionality

pub mod adapters;
pub mod http;
pub mod http3;
pub mod ssl;

#[cfg(unix)]
pub mod unix_socket;

// Re-exports
pub use http::{send_request_with_session, check_status, USER_AGENT_STRING, IntermediateResponse};
pub use http3::{send_http3_request, Http3Response, run_http3};

#[cfg(unix)]
pub use unix_socket::send_request as send_unix_socket_request;
