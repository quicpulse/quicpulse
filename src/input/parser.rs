//! Request item parser
//!
//! Parses CLI input strings like "Header:Value", "key=value", "file@path"
//! into strongly-typed InputItem variants.

use std::path::PathBuf;

use once_cell::sync::Lazy;

use super::{InputItem, FileUploadMeta};
use crate::errors::QuicpulseError;

/// Separator patterns - order doesn't matter as we sort by length at runtime
const SEPARATORS_UNSORTED: &[(&str, SeparatorKind)] = &[
    (":", SeparatorKind::Header),
    ("=", SeparatorKind::DataField),
    ("@", SeparatorKind::FileUpload),
    (";", SeparatorKind::EmptyHeader),
    (":=", SeparatorKind::JsonField),
    ("==", SeparatorKind::QueryParam),
    (":@", SeparatorKind::HeaderFile),
    ("=@", SeparatorKind::DataFieldFile),
    (":=@", SeparatorKind::JsonFieldFile),
    ("==@", SeparatorKind::QueryParamFile),
];

/// Separators sorted by length descending at runtime for longest-match precedence
static SEPARATORS: Lazy<Vec<(&'static str, SeparatorKind)>> = Lazy::new(|| {
    let mut sorted = SEPARATORS_UNSORTED.to_vec();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    sorted
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeparatorKind {
    Header,
    EmptyHeader,
    HeaderFile,
    QueryParam,
    QueryParamFile,
    DataField,
    DataFieldFile,
    JsonField,
    JsonFieldFile,
    FileUpload,
}

/// Parse a CLI request item string into an InputItem
pub fn parse(input: &str) -> Result<InputItem, QuicpulseError> {
    // Find the best matching separator
    let mut best: Option<(usize, &str, SeparatorKind)> = None;

    for (sep, kind) in SEPARATORS.iter() {
        if let Some(pos) = find_separator(input, sep, *kind) {
            match best {
                None => best = Some((pos, sep, *kind)),
                Some((best_pos, best_sep, _)) => {
                    // Earlier position wins, or longer separator at same position
                    if pos < best_pos || (pos == best_pos && sep.len() > best_sep.len()) {
                        best = Some((pos, sep, *kind));
                    }
                }
            }
        }
    }

    match best {
        Some((pos, sep, kind)) => build_item(input, pos, sep, kind),
        None => Err(QuicpulseError::Parse(format!(
            "Invalid request item '{}': no valid separator found. \
            Use formats like Header:Value, key=value, key:=json, file@path",
            input
        ))),
    }
}

/// Find a separator in the input, handling special cases
fn find_separator(input: &str, sep: &str, kind: SeparatorKind) -> Option<usize> {
    let pos = input.find(sep)?;

    // For single-char separators, check we're not actually matching a longer one
    match kind {
        SeparatorKind::Header => {
            // ':' should not match if followed by '=' or '@' (would be := or :@)
            if pos + 1 < input.len() {
                let next = input.as_bytes().get(pos + 1)?;
                if *next == b'=' || *next == b'@' {
                    return None;
                }
            }
            Some(pos)
        }
        SeparatorKind::DataField => {
            // '=' should not match if preceded by '=' (would be ==) or followed by '@'
            if pos > 0 && input.as_bytes().get(pos - 1) == Some(&b'=') {
                return None;
            }
            if pos + 1 < input.len() && input.as_bytes().get(pos + 1) == Some(&b'@') {
                return None;
            }
            Some(pos)
        }
        SeparatorKind::QueryParam => {
            // '==' should not match if followed by '@'
            if pos + 2 < input.len() && input.as_bytes().get(pos + 2) == Some(&b'@') {
                return None;
            }
            Some(pos)
        }
        SeparatorKind::JsonField => {
            // ':=' should not match if followed by '@'
            if pos + 2 < input.len() && input.as_bytes().get(pos + 2) == Some(&b'@') {
                return None;
            }
            Some(pos)
        }
        _ => Some(pos),
    }
}

/// Build an InputItem from parsed components
fn build_item(input: &str, pos: usize, sep: &str, kind: SeparatorKind) -> Result<InputItem, QuicpulseError> {
    let key = &input[..pos];
    let value = &input[pos + sep.len()..];

    if key.is_empty() && !matches!(kind, SeparatorKind::FileUpload) {
        return Err(QuicpulseError::Parse(format!(
            "Invalid request item '{}': empty key",
            input
        )));
    }

    match kind {
        SeparatorKind::Header => Ok(InputItem::Header {
            name: key.to_string(),
            value: value.to_string(),
        }),

        SeparatorKind::EmptyHeader => Ok(InputItem::EmptyHeader {
            name: key.to_string(),
        }),

        SeparatorKind::HeaderFile => Ok(InputItem::HeaderFile {
            name: key.to_string(),
            path: PathBuf::from(value),
        }),

        SeparatorKind::QueryParam => Ok(InputItem::QueryParam {
            name: key.to_string(),
            value: value.to_string(),
        }),

        SeparatorKind::QueryParamFile => Ok(InputItem::QueryParamFile {
            name: key.to_string(),
            path: PathBuf::from(value),
        }),

        SeparatorKind::DataField => Ok(InputItem::DataField {
            key: key.to_string(),
            value: value.to_string(),
        }),

        SeparatorKind::DataFieldFile => Ok(InputItem::DataFieldFile {
            key: key.to_string(),
            path: PathBuf::from(value),
        }),

        SeparatorKind::JsonField => {
            let json_value = serde_json::from_str(value)
                .map_err(|e| QuicpulseError::Parse(format!(
                    "Invalid JSON in '{}': {}",
                    input, e
                )))?;
            Ok(InputItem::JsonField {
                key: key.to_string(),
                value: json_value,
            })
        }

        SeparatorKind::JsonFieldFile => Ok(InputItem::JsonFieldFile {
            key: key.to_string(),
            path: PathBuf::from(value),
        }),

        SeparatorKind::FileUpload => {
            let (path, meta) = parse_file_path_with_meta(value);
            Ok(InputItem::FileUpload {
                field: key.to_string(),
                path,
                mime_type: meta.mime_type,
                filename: meta.filename,
            })
        }
    }
}

/// Parse file path with optional ;type=mime;filename=name suffix
fn parse_file_path_with_meta(s: &str) -> (PathBuf, FileUploadMeta) {
    let mut meta = FileUploadMeta::default();

    // Check if the entire path exists (handles paths with semicolons)
    let full_path = PathBuf::from(s);
    if full_path.exists() {
        return (full_path, meta);
    }

    // Try to split on semicolon for options
    if let Some(first_semi) = s.find(';') {
        let path_str = &s[..first_semi];
        let opts_str = &s[first_semi + 1..];

        for part in opts_str.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key.trim().to_lowercase().as_str() {
                    "type" => meta.mime_type = Some(value.trim().to_string()),
                    "filename" => meta.filename = Some(value.trim().to_string()),
                    _ => {}
                }
            } else if !part.is_empty() && meta.mime_type.is_none() {
                // Treat bare value as mime type
                meta.mime_type = Some(part.trim().to_string());
            }
        }

        return (PathBuf::from(path_str), meta);
    }

    (full_path, meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_header() {
        let item = parse("Content-Type:application/json").unwrap();
        assert!(matches!(item, InputItem::Header { name, value }
            if name == "Content-Type" && value == "application/json"));
    }

    #[test]
    fn test_parse_empty_header() {
        let item = parse("Accept;").unwrap();
        assert!(matches!(item, InputItem::EmptyHeader { name } if name == "Accept"));
    }

    #[test]
    fn test_parse_query_param() {
        let item = parse("search==rust").unwrap();
        assert!(matches!(item, InputItem::QueryParam { name, value }
            if name == "search" && value == "rust"));
    }

    #[test]
    fn test_parse_data_field() {
        let item = parse("username=john").unwrap();
        assert!(matches!(item, InputItem::DataField { key, value }
            if key == "username" && value == "john"));
    }

    #[test]
    fn test_parse_json_field() {
        let item = parse("count:=42").unwrap();
        if let InputItem::JsonField { key, value } = item {
            assert_eq!(key, "count");
            assert_eq!(value, json!(42));
        } else {
            panic!("Expected JsonField");
        }
    }

    #[test]
    fn test_parse_json_field_object() {
        let item = parse(r#"data:={"nested":true}"#).unwrap();
        if let InputItem::JsonField { key, value } = item {
            assert_eq!(key, "data");
            assert_eq!(value, json!({"nested": true}));
        } else {
            panic!("Expected JsonField");
        }
    }

    #[test]
    fn test_parse_file_upload() {
        let item = parse("avatar@/path/to/file.png").unwrap();
        if let InputItem::FileUpload { field, path, .. } = item {
            assert_eq!(field, "avatar");
            assert_eq!(path, PathBuf::from("/path/to/file.png"));
        } else {
            panic!("Expected FileUpload");
        }
    }

    #[test]
    fn test_parse_file_upload_with_type() {
        let item = parse("avatar@/path/to/file;type=image/png").unwrap();
        if let InputItem::FileUpload { field, mime_type, .. } = item {
            assert_eq!(field, "avatar");
            assert_eq!(mime_type, Some("image/png".to_string()));
        } else {
            panic!("Expected FileUpload");
        }
    }

    #[test]
    fn test_parse_header_file() {
        let item = parse("Token:@/path/to/token.txt").unwrap();
        if let InputItem::HeaderFile { name, path } = item {
            assert_eq!(name, "Token");
            assert_eq!(path, PathBuf::from("/path/to/token.txt"));
        } else {
            panic!("Expected HeaderFile");
        }
    }

    #[test]
    fn test_parse_data_file() {
        let item = parse("body=@/path/to/data.json").unwrap();
        if let InputItem::DataFieldFile { key, path } = item {
            assert_eq!(key, "body");
            assert_eq!(path, PathBuf::from("/path/to/data.json"));
        } else {
            panic!("Expected DataFieldFile");
        }
    }

    #[test]
    fn test_parse_query_file() {
        let item = parse("query==@/path/to/query.txt").unwrap();
        if let InputItem::QueryParamFile { name, path } = item {
            assert_eq!(name, "query");
            assert_eq!(path, PathBuf::from("/path/to/query.txt"));
        } else {
            panic!("Expected QueryParamFile");
        }
    }

    #[test]
    fn test_parse_json_file() {
        let item = parse("config:=@/path/to/config.json").unwrap();
        if let InputItem::JsonFieldFile { key, path } = item {
            assert_eq!(key, "config");
            assert_eq!(path, PathBuf::from("/path/to/config.json"));
        } else {
            panic!("Expected JsonFieldFile");
        }
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse("invalid-no-separator").is_err());
    }

    #[test]
    fn test_separator_precedence() {
        // :=@ should match as JsonFieldFile, not HeaderFile + @
        let item = parse("key:=@/path").unwrap();
        assert!(matches!(item, InputItem::JsonFieldFile { .. }));

        // == should match as QueryParam, not DataField twice
        let item = parse("key==value").unwrap();
        assert!(matches!(item, InputItem::QueryParam { .. }));
    }
}
