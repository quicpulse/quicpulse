//! gRPC codec for JSON to protobuf conversion
//!
//! This module provides utilities for converting between JSON and
//! protobuf wire format for dynamic gRPC calls.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde_json::Value as JsonValue;
use crate::errors::QuicpulseError;

/// Convert JSON to protobuf bytes
///
/// Note: This is a simplified implementation that uses JSON codec.
/// For true protobuf encoding, we would need message descriptors
/// from reflection or proto files.
pub fn json_to_proto(json: &JsonValue) -> Result<Bytes, QuicpulseError> {
    // For now, we use JSON codec which is supported by some gRPC servers
    // with gRPC-JSON transcoding enabled
    let bytes = serde_json::to_vec(json)
        .map_err(|e| QuicpulseError::Argument(format!("JSON serialization failed: {}", e)))?;

    Ok(Bytes::from(bytes))
}

/// Convert protobuf bytes to JSON
///
/// Note: This is a simplified implementation that assumes JSON codec.
pub fn proto_to_json(bytes: &[u8]) -> Result<JsonValue, QuicpulseError> {
    // Try to parse as JSON first (for JSON-transcoded responses)
    if let Ok(json) = serde_json::from_slice(bytes) {
        return Ok(json);
    }

    // If not JSON, return raw bytes as base64
    Ok(JsonValue::String(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        bytes
    )))
}

/// gRPC message frame header
/// Format: 1 byte compressed flag + 4 bytes message length (big endian)
const GRPC_HEADER_SIZE: usize = 5;

/// Frame a message for gRPC transport
pub fn frame_message(message: &[u8], compressed: bool) -> Bytes {
    let mut buf = BytesMut::with_capacity(GRPC_HEADER_SIZE + message.len());

    // Compressed flag (1 byte)
    buf.put_u8(if compressed { 1 } else { 0 });

    // Message length (4 bytes, big endian)
    buf.put_u32(message.len() as u32);

    // Message body
    buf.extend_from_slice(message);

    buf.freeze()
}

/// Unframe a gRPC message
pub fn unframe_message(mut buf: Bytes) -> Result<(bool, Bytes), QuicpulseError> {
    if buf.len() < GRPC_HEADER_SIZE {
        return Err(QuicpulseError::Argument("Message too short for gRPC frame".to_string()));
    }

    // Read compressed flag
    let compressed = buf.get_u8() != 0;

    // Read message length
    let length = buf.get_u32() as usize;

    // Verify we have enough data
    if buf.len() < length {
        return Err(QuicpulseError::Argument(format!(
            "Message truncated: expected {} bytes, got {}",
            length, buf.len()
        )));
    }

    // Extract message
    let message = buf.slice(..length);

    Ok((compressed, message))
}

/// A simple wire-format encoder for basic types
pub struct WireEncoder {
    buf: BytesMut,
}

impl WireEncoder {
    pub fn new() -> Self {
        Self { buf: BytesMut::new() }
    }

    /// Encode a varint
    pub fn write_varint(&mut self, mut value: u64) {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.buf.put_u8(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// Encode a field tag (field number + wire type)
    pub fn write_tag(&mut self, field_number: u32, wire_type: WireType) {
        let tag = (field_number << 3) | (wire_type as u32);
        self.write_varint(tag as u64);
    }

    /// Encode a string/bytes field
    pub fn write_length_delimited(&mut self, field_number: u32, data: &[u8]) {
        self.write_tag(field_number, WireType::LengthDelimited);
        self.write_varint(data.len() as u64);
        self.buf.extend_from_slice(data);
    }

    /// Encode a string field
    pub fn write_string(&mut self, field_number: u32, value: &str) {
        self.write_length_delimited(field_number, value.as_bytes());
    }

    /// Encode an int32/int64 field
    pub fn write_int(&mut self, field_number: u32, value: i64) {
        self.write_tag(field_number, WireType::Varint);
        self.write_varint(value as u64);
    }

    /// Encode a bool field
    pub fn write_bool(&mut self, field_number: u32, value: bool) {
        self.write_int(field_number, if value { 1 } else { 0 });
    }

    /// Encode a fixed64/double field
    pub fn write_fixed64(&mut self, field_number: u32, value: u64) {
        self.write_tag(field_number, WireType::Fixed64);
        self.buf.put_u64_le(value);
    }

    /// Encode a fixed32/float field
    pub fn write_fixed32(&mut self, field_number: u32, value: u32) {
        self.write_tag(field_number, WireType::Fixed32);
        self.buf.put_u32_le(value);
    }

    /// Get the encoded bytes
    pub fn finish(self) -> Bytes {
        self.buf.freeze()
    }
}

impl Default for WireEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Protobuf wire types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    StartGroup = 3,  // Deprecated
    EndGroup = 4,    // Deprecated
    Fixed32 = 5,
}

impl WireType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(WireType::Varint),
            1 => Some(WireType::Fixed64),
            2 => Some(WireType::LengthDelimited),
            3 => Some(WireType::StartGroup),
            4 => Some(WireType::EndGroup),
            5 => Some(WireType::Fixed32),
            _ => None,
        }
    }
}

/// A simple wire-format decoder
pub struct WireDecoder {
    buf: Bytes,
    pos: usize,
}

impl WireDecoder {
    pub fn new(buf: Bytes) -> Self {
        Self { buf, pos: 0 }
    }

    /// Check if there's more data
    pub fn has_remaining(&self) -> bool {
        self.pos < self.buf.len()
    }

    /// Read a varint
    pub fn read_varint(&mut self) -> Result<u64, QuicpulseError> {
        let mut result: u64 = 0;
        let mut shift = 0;

        loop {
            if self.pos >= self.buf.len() {
                return Err(QuicpulseError::Argument("Unexpected end of message".to_string()));
            }

            let byte = self.buf[self.pos];
            self.pos += 1;

            result |= ((byte & 0x7F) as u64) << shift;

            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err(QuicpulseError::Argument("Varint too long".to_string()));
            }
        }

        Ok(result)
    }

    /// Read a field tag
    pub fn read_tag(&mut self) -> Result<(u32, WireType), QuicpulseError> {
        let tag = self.read_varint()? as u32;
        let field_number = tag >> 3;
        let wire_type = WireType::from_u32(tag & 0x07)
            .ok_or_else(|| QuicpulseError::Argument("Invalid wire type".to_string()))?;
        Ok((field_number, wire_type))
    }

    /// Read length-delimited data
    pub fn read_length_delimited(&mut self) -> Result<Bytes, QuicpulseError> {
        let length = self.read_varint()? as usize;

        if self.pos + length > self.buf.len() {
            return Err(QuicpulseError::Argument("Length exceeds message size".to_string()));
        }

        let data = self.buf.slice(self.pos..self.pos + length);
        self.pos += length;

        Ok(data)
    }

    /// Read a string
    pub fn read_string(&mut self) -> Result<String, QuicpulseError> {
        let data = self.read_length_delimited()?;
        String::from_utf8(data.to_vec())
            .map_err(|e| QuicpulseError::Argument(format!("Invalid UTF-8: {}", e)))
    }

    /// Read fixed64
    pub fn read_fixed64(&mut self) -> Result<u64, QuicpulseError> {
        if self.pos + 8 > self.buf.len() {
            return Err(QuicpulseError::Argument("Unexpected end of message".to_string()));
        }

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.buf[self.pos..self.pos + 8]);
        self.pos += 8;

        Ok(u64::from_le_bytes(bytes))
    }

    /// Read fixed32
    pub fn read_fixed32(&mut self) -> Result<u32, QuicpulseError> {
        if self.pos + 4 > self.buf.len() {
            return Err(QuicpulseError::Argument("Unexpected end of message".to_string()));
        }

        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.buf[self.pos..self.pos + 4]);
        self.pos += 4;

        Ok(u32::from_le_bytes(bytes))
    }

    /// Skip a field based on wire type
    pub fn skip_field(&mut self, wire_type: WireType) -> Result<(), QuicpulseError> {
        match wire_type {
            WireType::Varint => {
                self.read_varint()?;
            }
            WireType::Fixed64 => {
                self.read_fixed64()?;
            }
            WireType::LengthDelimited => {
                self.read_length_delimited()?;
            }
            WireType::Fixed32 => {
                self.read_fixed32()?;
            }
            WireType::StartGroup | WireType::EndGroup => {
                return Err(QuicpulseError::Argument("Groups are deprecated".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_proto() {
        let json = serde_json::json!({"name": "test", "value": 42});
        let bytes = json_to_proto(&json).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_frame_message() {
        let message = b"hello";
        let framed = frame_message(message, false);

        assert_eq!(framed[0], 0); // not compressed
        assert_eq!(&framed[1..5], &[0, 0, 0, 5]); // length = 5
        assert_eq!(&framed[5..], b"hello");
    }

    #[test]
    fn test_unframe_message() {
        let framed = Bytes::from_static(&[0, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o']);
        let (compressed, message) = unframe_message(framed).unwrap();

        assert!(!compressed);
        assert_eq!(&message[..], b"hello");
    }

    #[test]
    fn test_wire_encoder_varint() {
        let mut encoder = WireEncoder::new();
        encoder.write_int(1, 150);
        let bytes = encoder.finish();

        // Field 1, wire type 0 (varint) = 0x08
        // 150 encoded as varint = 0x96 0x01
        assert_eq!(&bytes[..], &[0x08, 0x96, 0x01]);
    }

    #[test]
    fn test_wire_encoder_string() {
        let mut encoder = WireEncoder::new();
        encoder.write_string(2, "test");
        let bytes = encoder.finish();

        // Field 2, wire type 2 (length delimited) = 0x12
        // Length 4 = 0x04
        // "test" = 0x74 0x65 0x73 0x74
        assert_eq!(&bytes[..], &[0x12, 0x04, b't', b'e', b's', b't']);
    }

    #[test]
    fn test_wire_decoder() {
        let bytes = Bytes::from_static(&[0x08, 0x96, 0x01, 0x12, 0x04, b't', b'e', b's', b't']);
        let mut decoder = WireDecoder::new(bytes);

        // Read first field (int)
        let (field_num, wire_type) = decoder.read_tag().unwrap();
        assert_eq!(field_num, 1);
        assert_eq!(wire_type, WireType::Varint);
        let value = decoder.read_varint().unwrap();
        assert_eq!(value, 150);

        // Read second field (string)
        let (field_num, wire_type) = decoder.read_tag().unwrap();
        assert_eq!(field_num, 2);
        assert_eq!(wire_type, WireType::LengthDelimited);
        let value = decoder.read_string().unwrap();
        assert_eq!(value, "test");
    }
}
