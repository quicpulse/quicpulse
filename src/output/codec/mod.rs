//! Output codec implementations using tokio_util::codec::Decoder
//!
//! These codecs provide incremental decoding with proper error handling
//! and backpressure support for async streaming.

mod encoded;
mod pretty;
mod raw;

pub use encoded::EncodedCodec;
pub use pretty::PrettyCodec;
pub use raw::RawCodec;

use crate::output::error::StreamError;

/// Common trait for output codecs
pub trait OutputCodec {
    /// Get the content type this codec handles
    fn content_type(&self) -> Option<&str>;

    /// Whether this codec is suitable for binary data
    fn handles_binary(&self) -> bool;
}
