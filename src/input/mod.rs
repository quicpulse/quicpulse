//! Input parsing module
//!
//! Provides strongly-typed parsing of CLI request items like headers, data fields,
//! query parameters, and file uploads. Uses Rust enums for type safety instead of
//! string-based separator checking.

mod parser;

use std::path::PathBuf;
use serde_json::Value as JsonValue;

pub use parser::parse;

/// A parsed CLI input item with full type information
///
/// Each variant represents a different type of request modification.
/// The parser immediately classifies input into the correct variant,
/// enabling exhaustive pattern matching without string comparisons.
#[derive(Debug, Clone)]
pub enum InputItem {
    // =========================================================================
    // HEADERS
    // =========================================================================

    /// HTTP header: "Name:Value"
    Header { name: String, value: String },

    /// Empty HTTP header: "Name;"
    EmptyHeader { name: String },

    /// HTTP header from file: "Name:@path"
    HeaderFile { name: String, path: PathBuf },

    // =========================================================================
    // QUERY PARAMETERS
    // =========================================================================

    /// URL query parameter: "name==value"
    QueryParam { name: String, value: String },

    /// URL query parameter from file: "name==@path"
    QueryParamFile { name: String, path: PathBuf },

    // =========================================================================
    // DATA FIELDS (form or JSON depending on mode)
    // =========================================================================

    /// Data field: "key=value"
    /// In JSON mode: becomes {"key": "value"}
    /// In form mode: becomes key=value in body
    DataField { key: String, value: String },

    /// Data field from file: "key=@path"
    DataFieldFile { key: String, path: PathBuf },

    // =========================================================================
    // JSON FIELDS (always JSON, regardless of mode)
    // =========================================================================

    /// JSON field with parsed value: "key:=value"
    /// Value is parsed as JSON (number, bool, object, array, null)
    JsonField { key: String, value: JsonValue },

    /// JSON field from file: "key:=@path"
    JsonFieldFile { key: String, path: PathBuf },

    // =========================================================================
    // FILE UPLOADS
    // =========================================================================

    /// File upload: "field@path" or "field@path;type=mime;filename=name"
    FileUpload {
        field: String,
        path: PathBuf,
        mime_type: Option<String>,
        filename: Option<String>,
    },
}

impl InputItem {
    /// Parse a CLI argument string into an InputItem
    pub fn parse(input: &str) -> Result<Self, crate::errors::QuicpulseError> {
        parser::parse(input)
    }

    /// Check if this item contributes request data (triggers POST method)
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            InputItem::DataField { .. }
                | InputItem::DataFieldFile { .. }
                | InputItem::JsonField { .. }
                | InputItem::JsonFieldFile { .. }
                | InputItem::FileUpload { .. }
        )
    }

    /// Check if this item is a header
    pub fn is_header(&self) -> bool {
        matches!(
            self,
            InputItem::Header { .. }
                | InputItem::EmptyHeader { .. }
                | InputItem::HeaderFile { .. }
        )
    }

    /// Check if this item is a query parameter
    pub fn is_query(&self) -> bool {
        matches!(
            self,
            InputItem::QueryParam { .. } | InputItem::QueryParamFile { .. }
        )
    }

    /// Check if this item is a file upload
    pub fn is_file_upload(&self) -> bool {
        matches!(self, InputItem::FileUpload { .. })
    }

    /// Check if this item requires file reading
    pub fn requires_file_read(&self) -> bool {
        matches!(
            self,
            InputItem::HeaderFile { .. }
                | InputItem::QueryParamFile { .. }
                | InputItem::DataFieldFile { .. }
                | InputItem::JsonFieldFile { .. }
                | InputItem::FileUpload { .. }
        )
    }

    /// Get the key/name for this item
    pub fn key(&self) -> &str {
        match self {
            InputItem::Header { name, .. } => name,
            InputItem::EmptyHeader { name } => name,
            InputItem::HeaderFile { name, .. } => name,
            InputItem::QueryParam { name, .. } => name,
            InputItem::QueryParamFile { name, .. } => name,
            InputItem::DataField { key, .. } => key,
            InputItem::DataFieldFile { key, .. } => key,
            InputItem::JsonField { key, .. } => key,
            InputItem::JsonFieldFile { key, .. } => key,
            InputItem::FileUpload { field, .. } => field,
        }
    }

    /// Alias for is_data() for backward compatibility
    pub fn is_data_item(&self) -> bool {
        self.is_data()
    }

    /// Check if this is a JSON value item (:= or :=@)
    pub fn is_json_value(&self) -> bool {
        matches!(self, InputItem::JsonField { .. } | InputItem::JsonFieldFile { .. })
    }

    /// Get the string value for items that have one directly
    pub fn value(&self) -> Option<&str> {
        match self {
            InputItem::Header { value, .. } => Some(value),
            InputItem::EmptyHeader { .. } => Some(""),
            InputItem::QueryParam { value, .. } => Some(value),
            InputItem::DataField { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Get the JSON value if this is a JSON field
    pub fn json_value(&self) -> Option<&JsonValue> {
        match self {
            InputItem::JsonField { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Get the file path if this is a file-based item
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            InputItem::HeaderFile { path, .. } => Some(path),
            InputItem::QueryParamFile { path, .. } => Some(path),
            InputItem::DataFieldFile { path, .. } => Some(path),
            InputItem::JsonFieldFile { path, .. } => Some(path),
            InputItem::FileUpload { path, .. } => Some(path),
            _ => None,
        }
    }

    /// For file uploads, get the upload details
    pub fn file_upload_details(&self) -> Option<FileUploadDetails> {
        match self {
            InputItem::FileUpload { field, path, mime_type, filename } => Some(FileUploadDetails {
                field: field.clone(),
                path: path.clone(),
                mime_type: mime_type.clone(),
                filename: filename.clone(),
            }),
            _ => None,
        }
    }
}

/// Details for file upload items
#[derive(Debug, Clone)]
pub struct FileUploadDetails {
    pub field: String,
    pub path: PathBuf,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
}

/// Metadata for file uploads
#[derive(Debug, Clone, Default)]
pub struct FileUploadMeta {
    pub mime_type: Option<String>,
    pub filename: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_data() {
        assert!(InputItem::DataField {
            key: "k".into(),
            value: "v".into()
        }
        .is_data());
        assert!(InputItem::JsonField {
            key: "k".into(),
            value: JsonValue::Null
        }
        .is_data());
        assert!(!InputItem::Header {
            name: "n".into(),
            value: "v".into()
        }
        .is_data());
        assert!(!InputItem::QueryParam {
            name: "n".into(),
            value: "v".into()
        }
        .is_data());
    }

    #[test]
    fn test_is_header() {
        assert!(InputItem::Header {
            name: "n".into(),
            value: "v".into()
        }
        .is_header());
        assert!(InputItem::EmptyHeader { name: "n".into() }.is_header());
        assert!(!InputItem::DataField {
            key: "k".into(),
            value: "v".into()
        }
        .is_header());
    }

    #[test]
    fn test_key() {
        assert_eq!(
            InputItem::Header {
                name: "Content-Type".into(),
                value: "json".into()
            }
            .key(),
            "Content-Type"
        );
        assert_eq!(
            InputItem::DataField {
                key: "username".into(),
                value: "john".into()
            }
            .key(),
            "username"
        );
    }
}
