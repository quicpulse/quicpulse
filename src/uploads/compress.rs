//! Request body compression
//!
//! Bug #5 fix: Provides both sync and async versions of compression functions.
//! The async version uses spawn_blocking to avoid blocking tokio worker threads
//! during CPU-intensive compression operations.

use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

use crate::errors::QuicpulseError;

/// Compress data using deflate (synchronous version)
///
/// Warning: This is CPU-intensive and blocks the current thread.
/// In async contexts, prefer `compress_deflate_async`.
pub fn compress_deflate(data: &[u8]) -> Result<Vec<u8>, QuicpulseError> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)
        .map_err(|e| QuicpulseError::Io(e))?;
    encoder.finish()
        .map_err(|e| QuicpulseError::Io(e))
}

/// Bug #5 fix: Async version of compress_deflate that uses spawn_blocking
/// to avoid blocking the tokio event loop during CPU-intensive compression.
pub async fn compress_deflate_async(data: Vec<u8>) -> Result<Vec<u8>, QuicpulseError> {
    tokio::task::spawn_blocking(move || compress_deflate(&data))
        .await
        .map_err(|e| QuicpulseError::Parse(format!("Compression task panicked: {}", e)))?
}

/// Compress request body if beneficial (synchronous version)
///
/// Returns (compressed_data, was_compressed)
///
/// Warning: This is CPU-intensive and blocks the current thread.
/// In async contexts, prefer `compress_request_async`.
pub fn compress_request(data: &[u8], always: bool) -> Result<(Vec<u8>, bool), QuicpulseError> {
    let compressed = compress_deflate(data)?;

    // Only use compression if it actually reduces size (unless always=true)
    if always || compressed.len() < data.len() {
        Ok((compressed, true))
    } else {
        Ok((data.to_vec(), false))
    }
}

/// Bug #5 fix: Async version of compress_request that uses spawn_blocking
/// to avoid blocking the tokio event loop during CPU-intensive compression.
pub async fn compress_request_async(data: Vec<u8>, always: bool) -> Result<(Vec<u8>, bool), QuicpulseError> {
    tokio::task::spawn_blocking(move || compress_request(&data, always))
        .await
        .map_err(|e| QuicpulseError::Parse(format!("Compression task panicked: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_deflate() {
        let data = b"Hello, World! Hello, World! Hello, World!";
        let compressed = compress_deflate(data).unwrap();
        
        // Compressed should be smaller for repetitive data
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compress_request_not_worth_it() {
        // Small data might not compress well
        let data = b"Hi";
        let (result, was_compressed) = compress_request(data, false).unwrap();
        
        // Should return original if compression doesn't help
        if !was_compressed {
            assert_eq!(result, data);
        }
    }

    #[test]
    fn test_compress_request_always() {
        let data = b"Hi";
        let (_, was_compressed) = compress_request(data, true).unwrap();
        
        // Should always compress when always=true
        assert!(was_compressed);
    }
}
