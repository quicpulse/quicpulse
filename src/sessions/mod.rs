//! Session persistence for cookies, headers, and auth

pub mod cookies;
pub mod session;

pub use session::{Session, SessionAuth};
