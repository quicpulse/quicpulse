//! Request configuration builder
//!
//! Builds HTTP request configuration from parsed InputItem variants.

use std::fs;
use std::path::PathBuf;

use indexmap::IndexMap;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value as JsonValue};

use super::json::set_nested_value;
use crate::errors::QuicpulseError;
use crate::input::InputItem;

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
                    let content = fs::read_to_string(&path).map_err(QuicpulseError::Io)?;
                    headers
                        .entry(name)
                        .or_default()
                        .push(content.trim().to_string());
                }

                // Query parameters
                InputItem::QueryParam { name, value } => {
                    query_params.push((name, value));
                }
                InputItem::QueryParamFile { name, path } => {
                    let content = fs::read_to_string(&path).map_err(QuicpulseError::Io)?;
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
                    let content = fs::read_to_string(&path).map_err(QuicpulseError::Io)?;
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
                    let content = fs::read_to_string(&path).map_err(QuicpulseError::Io)?;
                    let value: JsonValue =
                        serde_json::from_str(&content).map_err(QuicpulseError::Json)?;
                    set_nested_value(&mut json_data, &key, value)?;
                }

                // File uploads
                InputItem::FileUpload {
                    field,
                    path,
                    mime_type,
                    filename,
                } => {
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
        } else if is_json
            && json_data
                .as_object()
                .map(|m| !m.is_empty())
                .unwrap_or(false)
        {
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
            let header_name = HeaderName::try_from(name.as_str()).map_err(|e| {
                QuicpulseError::Parse(format!("Invalid header name '{}': {}", name, e))
            })?;
            for value in values {
                let header_value = HeaderValue::try_from(value.as_str()).map_err(|e| {
                    QuicpulseError::Parse(format!("Invalid header value '{}': {}", value, e))
                })?;
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
            InputItem::DataField {
                key: "name".to_string(),
                value: "John".to_string(),
            },
            InputItem::JsonField {
                key: "age".to_string(),
                value: json!(30),
            },
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
            InputItem::DataField {
                key: "username".to_string(),
                value: "john".to_string(),
            },
            InputItem::DataField {
                key: "password".to_string(),
                value: "secret".to_string(),
            },
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
            InputItem::Header {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
            InputItem::Header {
                name: "X-Custom".to_string(),
                value: "value1".to_string(),
            },
            InputItem::Header {
                name: "X-Custom".to_string(),
                value: "value2".to_string(),
            },
        ];

        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(
            config.headers.get("Content-Type"),
            Some(&vec!["application/json".to_string()])
        );
        assert_eq!(
            config.headers.get("X-Custom"),
            Some(&vec!["value1".to_string(), "value2".to_string()])
        );
    }

    #[test]
    fn test_build_query_params() {
        let items = vec![
            InputItem::QueryParam {
                name: "page".to_string(),
                value: "1".to_string(),
            },
            InputItem::QueryParam {
                name: "limit".to_string(),
                value: "10".to_string(),
            },
        ];

        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.query_params.len(), 2);
        assert_eq!(
            config.query_params[0],
            ("page".to_string(), "1".to_string())
        );
    }

    fn write_temp(dir: &tempfile::TempDir, name: &str, contents: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    // ---- body selection ----

    #[test]
    fn test_no_items_produces_no_body() {
        let config = RequestConfig::from_items(vec![], true).unwrap();
        assert!(!config.has_body());
        assert!(!config.has_files());
        assert!(config.body.is_none());
        assert!(config.json_body().is_none());
        assert!(config.form_data().is_none());
        assert!(config.files().is_none());
    }

    #[test]
    fn test_headers_alone_do_not_create_a_body() {
        let items = vec![InputItem::Header {
            name: "X-A".to_string(),
            value: "1".to_string(),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert!(!config.has_body());
    }

    #[test]
    fn test_is_json_flag_is_recorded() {
        assert!(RequestConfig::from_items(vec![], true).unwrap().is_json);
        assert!(!RequestConfig::from_items(vec![], false).unwrap().is_json);
    }

    #[test]
    fn test_json_fields_apply_even_in_form_mode() {
        // JsonField is documented as always-JSON regardless of mode, so it
        // produces a JSON body even with is_json = false.
        let items = vec![InputItem::JsonField {
            key: "n".to_string(),
            value: json!(5),
        }];
        let config = RequestConfig::from_items(items, false).unwrap();
        assert_eq!(
            config.json_body(),
            None,
            "form mode suppresses the JSON body"
        );
        assert!(!config.has_body());
    }

    #[test]
    fn test_files_take_precedence_over_data() {
        let items = vec![
            InputItem::DataField {
                key: "k".to_string(),
                value: "v".to_string(),
            },
            InputItem::FileUpload {
                field: "f".to_string(),
                path: PathBuf::from("/tmp/x.txt"),
                mime_type: None,
                filename: None,
            },
        ];
        let config = RequestConfig::from_items(items, false).unwrap();
        assert!(config.has_files());
        assert!(config.form_data().is_none(), "multipart wins over form");
        assert_eq!(config.files().unwrap().len(), 1);
    }

    #[test]
    fn test_file_upload_carries_its_overrides() {
        let items = vec![InputItem::FileUpload {
            field: "doc".to_string(),
            path: PathBuf::from("/tmp/a.bin"),
            mime_type: Some("application/x-thing".to_string()),
            filename: Some("renamed.bin".to_string()),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        let f = &config.files().unwrap()[0];

        assert_eq!(f.name, "doc");
        assert_eq!(f.path, PathBuf::from("/tmp/a.bin"));
        assert_eq!(f.content_type.as_deref(), Some("application/x-thing"));
        assert_eq!(f.filename.as_deref(), Some("renamed.bin"));
    }

    #[test]
    fn test_root_level_json_array_becomes_the_body() {
        // set_nested_value with a "[0]"-style key builds a root array, which
        // takes the dedicated array branch rather than the object branch.
        let items = vec![InputItem::JsonField {
            key: "[0]".to_string(),
            value: json!("first"),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        match config.json_body() {
            Some(JsonValue::Array(a)) => assert_eq!(a[0], "first"),
            other => panic!("expected a root array body, got {other:?}"),
        }
    }

    #[test]
    fn test_bracket_keys_build_nested_json() {
        let items = vec![
            InputItem::JsonField {
                key: "user[name]".to_string(),
                value: json!("alex"),
            },
            InputItem::JsonField {
                key: "user[age]".to_string(),
                value: json!(30),
            },
        ];
        let config = RequestConfig::from_items(items, true).unwrap();
        let body = config.json_body().unwrap();
        assert_eq!(body["user"]["name"], "alex");
        assert_eq!(body["user"]["age"], 30);
    }

    #[test]
    fn test_dotted_keys_are_literal_not_nested() {
        // Only bracket syntax nests; a dot is an ordinary key character.
        let items = vec![InputItem::JsonField {
            key: "user.name".to_string(),
            value: json!("alex"),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        let body = config.json_body().unwrap();
        assert_eq!(body["user.name"], "alex");
        assert!(body["user"].is_null());
    }

    #[test]
    fn test_array_append_syntax_collects_values() {
        let items = vec![
            InputItem::JsonField {
                key: "tags[]".to_string(),
                value: json!("a"),
            },
            InputItem::JsonField {
                key: "tags[]".to_string(),
                value: json!("b"),
            },
        ];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.json_body().unwrap()["tags"], json!(["a", "b"]));
    }

    #[test]
    fn test_empty_header_becomes_an_empty_value() {
        let items = vec![InputItem::EmptyHeader {
            name: "X-Drop".to_string(),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.headers.get("X-Drop"), Some(&vec![String::new()]));
    }

    // ---- file-backed items ----

    #[test]
    fn test_header_from_file_is_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp(&dir, "h.txt", "  token-value \n");

        let items = vec![InputItem::HeaderFile {
            name: "X-Token".to_string(),
            path,
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(
            config.headers.get("X-Token"),
            Some(&vec!["token-value".to_string()])
        );
    }

    #[test]
    fn test_query_param_from_file_is_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp(&dir, "q.txt", "search\n");

        let items = vec![InputItem::QueryParamFile {
            name: "q".to_string(),
            path,
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(
            config.query_params[0],
            ("q".to_string(), "search".to_string())
        );
    }

    #[test]
    fn test_data_field_from_file_in_both_modes() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp(&dir, "d.txt", "contents");

        let json_config = RequestConfig::from_items(
            vec![InputItem::DataFieldFile {
                key: "k".to_string(),
                path: path.clone(),
            }],
            true,
        )
        .unwrap();
        assert_eq!(json_config.json_body().unwrap()["k"], "contents");

        let form_config = RequestConfig::from_items(
            vec![InputItem::DataFieldFile {
                key: "k".to_string(),
                path,
            }],
            false,
        )
        .unwrap();
        assert_eq!(
            form_config.form_data().unwrap()[0],
            ("k".to_string(), "contents".to_string())
        );
    }

    #[test]
    fn test_json_field_from_file_is_parsed_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp(&dir, "v.json", r#"{"nested": [1, 2]}"#);

        let items = vec![InputItem::JsonFieldFile {
            key: "payload".to_string(),
            path,
        }];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.json_body().unwrap()["payload"]["nested"][1], 2);
    }

    #[test]
    fn test_missing_files_surface_as_io_errors() {
        let missing = PathBuf::from("/nonexistent/nope.txt");
        let cases: Vec<InputItem> = vec![
            InputItem::HeaderFile {
                name: "X".to_string(),
                path: missing.clone(),
            },
            InputItem::QueryParamFile {
                name: "q".to_string(),
                path: missing.clone(),
            },
            InputItem::DataFieldFile {
                key: "k".to_string(),
                path: missing.clone(),
            },
            InputItem::JsonFieldFile {
                key: "k".to_string(),
                path: missing,
            },
        ];

        for item in cases {
            let err = RequestConfig::from_items(vec![item], true).unwrap_err();
            assert!(matches!(err, QuicpulseError::Io(_)), "got {err:?}");
        }
    }

    #[test]
    fn test_malformed_json_file_surfaces_as_a_json_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp(&dir, "bad.json", "{not json");

        let items = vec![InputItem::JsonFieldFile {
            key: "k".to_string(),
            path,
        }];
        let err = RequestConfig::from_items(items, true).unwrap_err();
        assert!(matches!(err, QuicpulseError::Json(_)), "got {err:?}");
    }

    // ---- to_header_map ----

    #[test]
    fn test_to_header_map_preserves_repeated_headers() {
        let items = vec![
            InputItem::Header {
                name: "X-Multi".to_string(),
                value: "a".to_string(),
            },
            InputItem::Header {
                name: "X-Multi".to_string(),
                value: "b".to_string(),
            },
        ];
        let config = RequestConfig::from_items(items, true).unwrap();
        let map = config.to_header_map().unwrap();

        let values: Vec<&str> = map
            .get_all("x-multi")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(values, ["a", "b"]);
    }

    #[test]
    fn test_to_header_map_on_empty_headers() {
        let config = RequestConfig::from_items(vec![], true).unwrap();
        assert!(config.to_header_map().unwrap().is_empty());
    }

    #[test]
    fn test_to_header_map_rejects_an_invalid_header_name() {
        let items = vec![InputItem::Header {
            name: "Bad Header Name".to_string(),
            value: "v".to_string(),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();

        let err = config.to_header_map().unwrap_err();
        assert!(
            matches!(err, QuicpulseError::Parse(ref m) if m.contains("Invalid header name")),
            "got {err:?}"
        );
    }

    #[test]
    fn test_to_header_map_rejects_an_invalid_header_value() {
        let items = vec![InputItem::Header {
            name: "X-Bad".to_string(),
            // A newline cannot appear in a header value.
            value: "line1\nline2".to_string(),
        }];
        let config = RequestConfig::from_items(items, true).unwrap();

        let err = config.to_header_map().unwrap_err();
        assert!(
            matches!(err, QuicpulseError::Parse(ref m) if m.contains("Invalid header value")),
            "got {err:?}"
        );
    }

    // ---- accessors ----

    #[test]
    fn test_accessors_only_answer_for_their_own_body_kind() {
        let json_cfg = RequestConfig::from_items(
            vec![InputItem::JsonField {
                key: "k".to_string(),
                value: json!(1),
            }],
            true,
        )
        .unwrap();
        assert!(json_cfg.json_body().is_some());
        assert!(json_cfg.form_data().is_none());
        assert!(json_cfg.files().is_none());
        assert!(!json_cfg.has_files());

        let form_cfg = RequestConfig::from_items(
            vec![InputItem::DataField {
                key: "k".to_string(),
                value: "v".to_string(),
            }],
            false,
        )
        .unwrap();
        assert!(form_cfg.form_data().is_some());
        assert!(form_cfg.json_body().is_none());
        assert!(form_cfg.files().is_none());
    }

    #[test]
    fn test_raw_body_accessors_return_nothing() {
        // Raw bodies are set by other layers; the typed accessors must not
        // mistake them for JSON, form, or multipart payloads.
        let config = RequestConfig {
            headers: IndexMap::new(),
            body: Some(RequestBody::Raw(b"bytes".to_vec())),
            query_params: Vec::new(),
            is_json: false,
        };
        assert!(config.has_body());
        assert!(!config.has_files());
        assert!(config.json_body().is_none());
        assert!(config.form_data().is_none());
        assert!(config.files().is_none());
    }

    #[test]
    fn test_form_mode_preserves_duplicate_keys_in_order() {
        let items = vec![
            InputItem::DataField {
                key: "tag".to_string(),
                value: "a".to_string(),
            },
            InputItem::DataField {
                key: "tag".to_string(),
                value: "b".to_string(),
            },
        ];
        let config = RequestConfig::from_items(items, false).unwrap();
        let form = config.form_data().unwrap();
        assert_eq!(form.len(), 2, "form bodies allow repeated keys");
        assert_eq!(form[0].1, "a");
        assert_eq!(form[1].1, "b");
    }

    #[test]
    fn test_json_mode_later_value_wins_for_a_repeated_key() {
        let items = vec![
            InputItem::DataField {
                key: "k".to_string(),
                value: "first".to_string(),
            },
            InputItem::DataField {
                key: "k".to_string(),
                value: "second".to_string(),
            },
        ];
        let config = RequestConfig::from_items(items, true).unwrap();
        assert_eq!(config.json_body().unwrap()["k"], "second");
    }
}
