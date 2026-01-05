//! WebSocket message encoding/decoding utilities

use crate::errors::QuicpulseError;
use super::types::BinaryMode;

/// Encode binary data to string representation
pub fn encode_binary(data: &[u8], mode: BinaryMode) -> String {
    match mode {
        BinaryMode::Hex => hex::encode(data),
        BinaryMode::Base64 => base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
    }
}

/// Decode string representation to binary data
pub fn decode_binary(s: &str, mode: BinaryMode) -> Result<Vec<u8>, QuicpulseError> {
    match mode {
        BinaryMode::Hex => {
            hex::decode(s.trim())
                .map_err(|e| QuicpulseError::WebSocket(format!("Invalid hex: {}", e)))
        }
        BinaryMode::Base64 => {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s.trim())
                .map_err(|e| QuicpulseError::WebSocket(format!("Invalid base64: {}", e)))
        }
    }
}

/// Try to detect if a string is valid JSON and pretty-print it
pub fn format_text_message(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| text.to_string())
    } else {
        text.to_string()
    }
}

/// Format binary data for display
pub fn format_binary_message(data: &[u8], mode: Option<BinaryMode>) -> String {
    let mode = mode.unwrap_or(BinaryMode::Hex);
    let encoded = encode_binary(data, mode);

    // Add length info for large messages
    if data.len() > 100 {
        format!("[{} bytes] {}", data.len(), encoded)
    } else {
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_hex() {
        let data = b"Hello";
        assert_eq!(encode_binary(data, BinaryMode::Hex), "48656c6c6f");
    }

    #[test]
    fn test_encode_base64() {
        let data = b"Hello";
        assert_eq!(encode_binary(data, BinaryMode::Base64), "SGVsbG8=");
    }

    #[test]
    fn test_decode_hex() {
        let result = decode_binary("48656c6c6f", BinaryMode::Hex).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_base64() {
        let result = decode_binary("SGVsbG8=", BinaryMode::Base64).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_format_json() {
        let json = r#"{"key":"value"}"#;
        let formatted = format_text_message(json);
        assert!(formatted.contains("\"key\""));
        assert!(formatted.contains("\"value\""));
    }

    #[test]
    fn test_format_non_json() {
        let text = "plain text";
        assert_eq!(format_text_message(text), "plain text");
    }
}
