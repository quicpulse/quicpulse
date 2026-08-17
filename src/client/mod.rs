//! HTTP client functionality

pub mod adapters;
pub mod http;
pub mod http3;
pub mod ssl;

#[cfg(unix)]
pub mod unix_socket;

// Re-exports
pub use http::{check_status, send_request_with_session, IntermediateResponse, USER_AGENT_STRING};
pub use http3::{run_http3, send_http3_request, Http3Response};

#[cfg(unix)]
pub use unix_socket::send_request as send_unix_socket_request;
