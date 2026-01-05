//! Chunked transfer encoding support

use bytes::Bytes;
use std::io::Read;

/// Default chunk size for uploads (100KB)
pub const CHUNK_SIZE: usize = 100 * 1024;

/// Iterator that yields chunks of data
pub struct ChunkedReader<R: Read> {
    reader: R,
    chunk_size: usize,
    finished: bool,
}

impl<R: Read> ChunkedReader<R> {
    /// Create a new chunked reader
    pub fn new(reader: R) -> Self {
        Self::with_chunk_size(reader, CHUNK_SIZE)
    }

    /// Create a chunked reader with custom chunk size
    pub fn with_chunk_size(reader: R, chunk_size: usize) -> Self {
        Self {
            reader,
            chunk_size,
            finished: false,
        }
    }
}

impl<R: Read> Iterator for ChunkedReader<R> {
    type Item = std::io::Result<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let mut buffer = vec![0u8; self.chunk_size];
        
        match self.reader.read(&mut buffer) {
            Ok(0) => {
                self.finished = true;
                None
            }
            Ok(n) => {
                buffer.truncate(n);
                Some(Ok(Bytes::from(buffer)))
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

/// Wrapper for streaming uploads with progress callback
pub struct ChunkedUploadStream<R: Read, F: FnMut(usize)> {
    reader: ChunkedReader<R>,
    callback: F,
}

impl<R: Read, F: FnMut(usize)> ChunkedUploadStream<R, F> {
    /// Create a new chunked upload stream with progress callback
    pub fn new(reader: R, callback: F) -> Self {
        Self {
            reader: ChunkedReader::new(reader),
            callback,
        }
    }
}

impl<R: Read, F: FnMut(usize)> Iterator for ChunkedUploadStream<R, F> {
    type Item = std::io::Result<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Some(Ok(bytes)) => {
                (self.callback)(bytes.len());
                Some(Ok(bytes))
            }
            other => other,
        }
    }
}
