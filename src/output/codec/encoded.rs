//! Encoded output codec with encoding detection
//!
//! Decodes bytes to UTF-8 text with proper encoding handling.

use bytes::{Buf, BytesMut};
use encoding_rs::{Encoding, UTF_8};
use tokio_util::codec::Decoder;

use crate::output::error::StreamError;

/// Maximum buffer size before forcing a decode (1MB)
const MAX_BUFFER_SIZE: usize = 1024 * 1024;

/// Codec that decodes bytes to text lines with encoding support
pub struct EncodedCodec {
    /// Character encoding to use
    encoding: &'static Encoding,
    /// Accumulated incomplete line
    incomplete_line: String,
    /// Whether we've finished decoding
    finished: bool,
}

impl EncodedCodec {
    /// Create a new codec with UTF-8 encoding
    pub fn new() -> Self {
        Self {
            encoding: UTF_8,
            incomplete_line: String::new(),
            finished: false,
        }
    }

    /// Create with specified encoding
    pub fn with_encoding(encoding_name: &str) -> Self {
        let encoding = Encoding::for_label(encoding_name.as_bytes())
            .unwrap_or(UTF_8);
        Self {
            encoding,
            incomplete_line: String::new(),
            finished: false,
        }
    }

    /// Decode bytes to string using the configured encoding
    fn decode_bytes(&self, bytes: &[u8]) -> Result<String, StreamError> {
        let (text, _, had_errors) = self.encoding.decode(bytes);
        if had_errors {
            // Still return the text, but could log warning
        }
        Ok(text.into_owned())
    }
}

impl Default for EncodedCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for EncodedCodec {
    type Item = String;
    type Error = StreamError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // Prevent unbounded memory growth
        if src.len() > MAX_BUFFER_SIZE {
            // Force decode of what we have
            let bytes = src.split().freeze();
            let text = self.decode_bytes(&bytes)?;

            // Try to find a complete line
            if let Some(newline_pos) = text.find('\n') {
                let line = format!("{}{}\n", self.incomplete_line, &text[..newline_pos]);
                self.incomplete_line = text[newline_pos + 1..].to_string();
                return Ok(Some(line));
            } else {
                self.incomplete_line.push_str(&text);
                return Ok(None);
            }
        }

        // Decode all available bytes
        let text = self.decode_bytes(src)?;
        src.clear();

        // Prepend any incomplete line from previous decode
        let full_text = if self.incomplete_line.is_empty() {
            text
        } else {
            let mut combined = std::mem::take(&mut self.incomplete_line);
            combined.push_str(&text);
            combined
        };

        // Find complete lines
        if let Some(newline_pos) = full_text.rfind('\n') {
            let complete = &full_text[..=newline_pos];
            self.incomplete_line = full_text[newline_pos + 1..].to_string();
            Ok(Some(complete.to_string()))
        } else {
            // No complete line yet
            self.incomplete_line = full_text;
            Ok(None)
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode any remaining bytes
        if !src.is_empty() {
            let text = self.decode_bytes(src)?;
            src.clear();
            self.incomplete_line.push_str(&text);
        }

        // Return any remaining incomplete line
        if !self.incomplete_line.is_empty() && !self.finished {
            self.finished = true;
            let line = std::mem::take(&mut self.incomplete_line);
            // Add newline if missing
            if line.ends_with('\n') {
                Ok(Some(line))
            } else {
                Ok(Some(format!("{}\n", line)))
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_line() {
        let mut codec = EncodedCodec::new();
        let mut buf = BytesMut::from("Hello, World!\n");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, Some("Hello, World!\n".to_string()));
    }

    #[test]
    fn test_decode_multiple_lines() {
        let mut codec = EncodedCodec::new();
        let mut buf = BytesMut::from("Line 1\nLine 2\n");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, Some("Line 1\nLine 2\n".to_string()));
    }

    #[test]
    fn test_decode_incomplete_line() {
        let mut codec = EncodedCodec::new();
        let mut buf = BytesMut::from("Incomplete");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, None);

        // Complete the line at EOF
        let mut empty = BytesMut::new();
        let result = codec.decode_eof(&mut empty).unwrap();
        assert_eq!(result, Some("Incomplete\n".to_string()));
    }
}
