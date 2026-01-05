//! Encoded output stream with encoding detection and Unicode handling

use encoding_rs::{Encoding, UTF_8};

/// Maximum data size to process at once (100MB)
/// Larger data should be streamed in chunks at a higher level
const MAX_DATA_SIZE: usize = 100 * 1024 * 1024;

/// Encoded stream that handles character encoding
/// Iterates over lines without double allocation
#[derive(Debug)]
pub struct EncodedStream {
    /// Decoded text content
    text: String,
    /// Current byte position for line iteration
    position: usize,
}

impl EncodedStream {
    /// Create from bytes, detecting or using specified encoding
    /// Returns error message if data is too large
    pub fn new(data: &[u8], encoding_override: Option<&str>) -> Self {
        // Warn and truncate if data is too large to prevent OOM
        let data = if data.len() > MAX_DATA_SIZE {
            eprintln!("Warning: Response body truncated to {} MB to prevent memory exhaustion",
                     MAX_DATA_SIZE / 1024 / 1024);
            &data[..MAX_DATA_SIZE]
        } else {
            data
        };

        let (text, _) = decode_with_encoding(data, encoding_override);

        Self {
            text,
            position: 0,
        }
    }

    /// Check if content contains binary data (NUL bytes)
    pub fn is_binary(data: &[u8]) -> bool {
        data.contains(&0)
    }

    /// Get the full text content
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Iterator for EncodedStream {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.text.len() {
            return None;
        }

        // Find next newline from current position
        let remaining = &self.text[self.position..];
        match remaining.find('\n') {
            Some(idx) => {
                // Include the newline in the output
                let line = &remaining[..=idx];
                self.position += idx + 1;
                Some(line.to_string())
            }
            None => {
                // Last line without trailing newline
                if remaining.is_empty() {
                    None
                } else {
                    self.position = self.text.len();
                    Some(format!("{}\n", remaining))
                }
            }
        }
    }
}

/// Decode bytes with given or detected encoding
fn decode_with_encoding(data: &[u8], encoding_name: Option<&str>) -> (String, &'static Encoding) {
    // Use specified encoding or detect
    let encoding = encoding_name
        .and_then(|name| Encoding::for_label(name.as_bytes()))
        .unwrap_or(UTF_8);

    let (text, _, had_errors) = encoding.decode(data);
    
    if had_errors {
        // Replace errors with replacement character
        (text.into_owned(), encoding)
    } else {
        (text.into_owned(), encoding)
    }
}

/// Detect encoding from Content-Type header
pub fn encoding_from_content_type(content_type: &str) -> Option<&str> {
    // Look for charset=...
    content_type
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            if part.to_lowercase().starts_with("charset=") {
                Some(part[8..].trim_matches('"').trim())
            } else {
                None
            }
        })
}
