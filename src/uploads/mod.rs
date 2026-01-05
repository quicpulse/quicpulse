//! Upload handling (multipart, chunked, compression)

pub mod chunked;
pub mod compress;
pub mod multipart;

pub use compress::compress_request;
