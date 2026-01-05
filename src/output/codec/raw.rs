//! Raw binary codec
//!
//! Passes bytes through without transformation, with chunking support.

use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

use crate::output::error::StreamError;

/// Default chunk size for raw output (64KB)
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Codec that passes bytes through unchanged
pub struct RawCodec {
    /// Chunk size for output
    chunk_size: usize,
    /// Whether we've finished
    finished: bool,
}

impl RawCodec {
    /// Create a new raw codec with default chunk size
    pub fn new() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            finished: false,
        }
    }

    /// Create with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            finished: false,
        }
    }
}

impl Default for RawCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for RawCodec {
    type Item = Bytes;
    type Error = StreamError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // Return chunks of the configured size
        if src.len() >= self.chunk_size {
            let chunk = src.split_to(self.chunk_size);
            Ok(Some(chunk.freeze()))
        } else {
            // Not enough data for a full chunk, wait for more
            Ok(None)
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !src.is_empty() && !self.finished {
            self.finished = true;
            let chunk = src.split().freeze();
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_decode() {
        let mut codec = RawCodec::with_chunk_size(10);
        let mut buf = BytesMut::from(&b"Hello, World!"[..]);

        // First decode returns first chunk
        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, Some(Bytes::from_static(b"Hello, Wor")));

        // Second decode at EOF returns remainder
        let result = codec.decode_eof(&mut buf).unwrap();
        assert_eq!(result, Some(Bytes::from_static(b"ld!")));
    }
}
