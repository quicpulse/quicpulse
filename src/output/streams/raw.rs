//! Raw output stream with no processing

use bytes::Bytes;

/// Chunk size for raw output (64KB - standard pipe buffer size)
pub const RAW_CHUNK_SIZE: usize = 64 * 1024;

/// Raw stream that passes data through unchanged
#[derive(Debug)]
pub struct RawStream {
    /// Data buffer
    data: Vec<u8>,
    /// Current position
    position: usize,
    /// Chunk size
    chunk_size: usize,
}

impl RawStream {
    /// Create a new raw stream from bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            position: 0,
            chunk_size: RAW_CHUNK_SIZE,
        }
    }

    /// Create with custom chunk size
    pub fn with_chunk_size(data: Vec<u8>, chunk_size: usize) -> Self {
        Self {
            data,
            position: 0,
            chunk_size,
        }
    }
}

impl Iterator for RawStream {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.data.len() {
            return None;
        }

        let end = (self.position + self.chunk_size).min(self.data.len());
        let chunk = Bytes::from(self.data[self.position..end].to_vec());
        self.position = end;

        Some(chunk)
    }
}
