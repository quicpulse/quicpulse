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
    encoder.write_all(data).map_err(QuicpulseError::Io)?;
    encoder.finish().map_err(QuicpulseError::Io)
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
pub async fn compress_request_async(
    data: Vec<u8>,
    always: bool,
) -> Result<(Vec<u8>, bool), QuicpulseError> {
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

    /// Inflate deflate-compressed bytes so tests can prove the payload survives.
    fn inflate(compressed: &[u8]) -> Vec<u8> {
        use flate2::read::DeflateDecoder;
        use std::io::Read;

        let mut out = Vec::new();
        DeflateDecoder::new(compressed)
            .read_to_end(&mut out)
            .expect("compressed data should inflate");
        out
    }

    #[test]
    fn test_compress_deflate_round_trips() {
        let data = b"The quick brown fox jumps over the lazy dog, repeatedly. ".repeat(20);
        let compressed = compress_deflate(&data).unwrap();
        assert_eq!(inflate(&compressed), data);
    }

    #[test]
    fn test_compress_deflate_handles_empty_input() {
        let compressed = compress_deflate(b"").unwrap();
        assert_eq!(inflate(&compressed), Vec::<u8>::new());
    }

    #[test]
    fn test_compress_deflate_handles_binary_data() {
        let data: Vec<u8> = (0..=255u8).cycle().take(3000).collect();
        let compressed = compress_deflate(&data).unwrap();
        assert_eq!(inflate(&compressed), data);
    }

    #[test]
    fn test_compress_request_returns_compressed_payload_when_it_helps() {
        let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let (out, was_compressed) = compress_request(data, false).unwrap();

        assert!(was_compressed, "repetitive data should compress");
        assert!(out.len() < data.len());
        assert_eq!(inflate(&out), data.to_vec());
    }

    #[test]
    fn test_compress_request_returns_the_original_when_compression_grows_it() {
        // Two bytes cannot be shrunk by deflate's framing overhead.
        let data = b"Hi";
        let (out, was_compressed) = compress_request(data, false).unwrap();
        assert!(!was_compressed);
        assert_eq!(out, data.to_vec(), "uncompressed result must be verbatim");
    }

    #[test]
    fn test_compress_request_always_still_produces_valid_deflate() {
        let data = b"Hi";
        let (out, was_compressed) = compress_request(data, true).unwrap();
        assert!(was_compressed);
        assert_eq!(inflate(&out), data.to_vec());
    }

    #[test]
    fn test_compress_request_empty_input() {
        let (out, was_compressed) = compress_request(b"", false).unwrap();
        // Nothing can be smaller than zero bytes, so the original is kept.
        assert!(!was_compressed);
        assert!(out.is_empty());
    }

    // ---- async wrappers ----

    #[tokio::test]
    async fn test_compress_deflate_async_matches_sync() {
        let data = b"payload payload payload payload".to_vec();
        let via_async = compress_deflate_async(data.clone()).await.unwrap();
        assert_eq!(via_async, compress_deflate(&data).unwrap());
        assert_eq!(inflate(&via_async), data);
    }

    #[tokio::test]
    async fn test_compress_request_async_matches_sync() {
        let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec();

        let (async_out, async_flag) = compress_request_async(data.clone(), false).await.unwrap();
        let (sync_out, sync_flag) = compress_request(&data, false).unwrap();

        assert_eq!(async_flag, sync_flag);
        assert_eq!(async_out, sync_out);
        assert!(async_flag);
    }

    #[tokio::test]
    async fn test_compress_request_async_honours_always() {
        let (out, was_compressed) = compress_request_async(b"Hi".to_vec(), true).await.unwrap();
        assert!(was_compressed);
        assert_eq!(inflate(&out), b"Hi".to_vec());
    }
}
