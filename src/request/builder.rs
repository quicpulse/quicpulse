//! Request configuration builder
//!
//! Builds HTTP request configuration from parsed InputItem variants.

use std::fs;
use std::path::PathBuf;

use indexmap::IndexMap;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value as JsonValue};

use crate::errors::QuicpulseError;
use crate::input::InputItem;
use super::json::set_nested_value;

/// Configured request ready to be sent
#[derive(Debug, Clone)]
pub struct RequestConfig {
    /// HTTP headers (supports multiple values per name)
    pub headers: IndexMap<String, Vec<String>>,
    /// Request body
    pub body: Option<RequestBody>,
    /// Query parameters
    pub query_params: Vec<(String, String)>,
    /// Whether JSON mode is enabled
    pub is_json: bool,
}

/// Request body variants
#[derive(Debug, Clone)]
pub enum RequestBody {
    /// JSON body
    Json(JsonValue),
    /// Form-urlencoded body
    Form(Vec<(String, String)>),
    /// Multipart form data
    Multipart(Vec<FileField>),
    /// Raw bytes
    Raw(Vec<u8>),
}

/// File field for multipart uploads
#[derive(Debug, Clone)]
pub struct FileField {
    /// Form field name
    pub name: String,
    /// File path
    pub path: PathBuf,
    /// Override filename
    pub filename: Option<String>,
    /// Override content type
    pub content_type: Option<String>,
}

impl RequestConfig {
    /// Build request configuration from parsed input items
    pub fn from_items(items: Vec<InputItem>, is_json: bool) -> Result<Self, QuicpulseError> {
        let mut headers: IndexMap<String, Vec<String>> = IndexMap::new();
        let mut json_data = json!({});
        let mut form_data: Vec<(String, String)> = Vec::new();
        let mut files: Vec<FileField> = Vec::new();
        let mut query_params: Vec<(String, String)> = Vec::new();

        for item in items {
            match item {
                // Headers
                InputItem::Header { name, value } => {
                    headers.entry(name).or_default().push(value);
                }
                InputItem::EmptyHeader { name } => {
                    headers.entry(name).or_default().push(String::new());
                }
                InputItem::HeaderFile { name, path } => {
                    let content = fs::read_to_string(&path)
                        .map_err(QuicpulseError::Io)?;
                    headers.entry(name).or_default().push(content.trim().to_string());
                }

                // Query parameters
                InputItem::QueryParam { name, value } => {
                    query_params.push((name, value));
                }
                InputItem::QueryParamFile { name, path } => {
                    let content = fs::read_to_string(&path)
                        .map_err(QuicpulseError::Io)?;
                    query_params.push((name, content.trim().to_string()));
                }

                // Data fields
                InputItem::DataField { key, value } => {
                    if is_json {
                        set_nested_value(&mut json_data, &key, JsonValue::String(value))?;
                    } else {
                        form_data.push((key, value));
                    }
                }
                InputItem::DataFieldFile { key, path } => {
                    let content = fs::read_to_string(&path)
                        .map_err(QuicpulseError::Io)?;
                    if is_json {
                        set_nested_value(&mut json_data, &key, JsonValue::String(content))?;
                    } else {
                        form_data.push((key, content));
                    }
                }

                // JSON fields (always JSON regardless of mode)
                InputItem::JsonField { key, value } => {
                    set_nested_value(&mut json_data, &key, value)?;
                }
                InputItem::JsonFieldFile { key, path } => {
                    let content = fs::read_to_string(&path)
                        .map_err(QuicpulseError::Io)?;
                    let value: JsonValue = serde_json::from_str(&content)
                        .map_err(QuicpulseError::Json)?;
                    set_nested_value(&mut json_data, &key, value)?;
                }

                // File uploads
                InputItem::FileUpload { field, path, mime_type, filename } => {
                    files.push(FileField {
                        name: field,
                        path,
                        filename,
                        content_type: mime_type,
                    });
                }
            }
        }

        // Determine body type
        let body = if !files.is_empty() {
            // Multipart if we have files
            // Include form data as text fields in multipart
            let fields = files;
            // Note: form_data would need to be handled separately for multipart
            // For now, files take precedence
            Some(RequestBody::Multipart(fields))
        } else if is_json && json_data.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
            Some(RequestBody::Json(json_data))
        } else if !form_data.is_empty() {
            Some(RequestBody::Form(form_data))
        } else if json_data.is_array() && !json_data.as_array().unwrap().is_empty() {
            // Handle root-level arrays
            Some(RequestBody::Json(json_data))
        } else {
            None
        };

        Ok(RequestConfig {
            headers,
            body,
            query_params,
            is_json,
        })
    }

    /// Check if there is any request body
    pub fn has_body(&self) -> bool {
        self.body.is_some()
    }

    /// Check if there are any files to upload
    pub fn has_files(&self) -> bool {
        matches!(&self.body, Some(RequestBody::Multipart(_)))
    }

    /// Convert headers to reqwest HeaderMap
    pub fn to_header_map(&self) -> Result<HeaderMap, QuicpulseError> {
        let mut map = HeaderMap::new();
        for (name, values) in &self.headers {
            let header_name = HeaderName::try_from(name.as_str())
                .map_err(|e| QuicpulseError::Parse(format!("Invalid header name '{}': {}", name, e)))?;
            for value in values {
                let header_value = HeaderValue::try_from(value.as_str())
                    .map_err(|e| QuicpulseError::Parse(format!("Invalid header value '{}': {}", value, e)))?;
                map.append(header_name.clone(), header_value);
            }
        }
        Ok(map)
    }

    /// Get JSON body if present
    pub fn json_body(&self) -> Option<&JsonValue> {
        match &self.body {
            Some(RequestBody::Json(v)) => Some(v),
            _ => None,
        }
    }

    /// Get form data if present
    pub fn form_data(&self) -> Option<&[(String, String)]> {
        match &self.body {
            Some(RequestBody::Form(v)) => Some(v),
            _ => None,
        }
    }

    /// Get file fields if present
    pub fn files(&self) -> Option<&[FileField]> {
        match &self.body {
            Some(RequestBody::Multipart(v)) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_json_request() {
        let items = vec![
            InputItem::DataField { key: "name".to_string(), value: "John".to_string() },
            InputItem::JsonField { key: "age".to_string(), value: json!(30) },
        ];

        let config = RequestConfig::from_items(items, true).unwrap();
        assert!(config.has_body());

        if let Some(RequestBody::Json(data)) = &config.body {
            assert_eq!(data["name"], "John");
            assert_eq!(data["age"], 30);
        } else {
            panic!("Expected JSON body");
        }
    }

    #[test]
    fn test_build_form_request() {
        let items = vec![
            InputItem::DataField { key: "username".to_string(), value: "john".to_string() },
            InputItem::DataField { key: "password".to_string(), value: "secret".to_string() },
        ];

        let config = RequestConfig::from_items(items, false).unwrap();
        assert!(config.has_body());

        if let Some(RequestBody::Form(data)) = &config.body {
            assert_eq!(data.len(), 2);
            assert_eq!(data[0], ("username".to_string(), "john".to_string()));
        } else {
            panic!("Expected form body");
        }
    }

    #[test]
    fn test_build_headers() {
        let items = vec![
            InputItem::Header { name: "Content-Type".to_string(), value: "application/json".to_string() },
            InputItem::Header { name: "X-Custom".to_string(), value: "value1".to_string() },
            InputItem::Header { name: "X-Custom".to_string(), value: "value2".to_string() },
        ];

        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.headers.get("Content-Type"), Some(&vec!["application/json".to_string()]));
        assert_eq!(config.headers.get("X-Custom"), Some(&vec!["value1".to_string(), "value2".to_string()]));
    }

    #[test]
    fn test_build_query_params() {
        let items = vec![
            InputItem::QueryParam { name: "page".to_string(), value: "1".to_string() },
            InputItem::QueryParam { name: "limit".to_string(), value: "10".to_string() },
        ];

        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.query_params.len(), 2);
        assert_eq!(config.query_params[0], ("page".to_string(), "1".to_string()));
    }
}
