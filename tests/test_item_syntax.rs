//! Exhaustive unit and integration tests for Request Item syntax and parsing

use quicpulse::input::InputItem;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_parse_headers() {
    let item = InputItem::parse("X-Custom-Header:MyValue").unwrap();
    assert!(item.is_header());
    assert!(!item.is_data());
    assert!(!item.is_query());
    if let InputItem::Header { name, value } = item {
        assert_eq!(name, "X-Custom-Header");
        assert_eq!(value, "MyValue");
    } else {
        panic!("Expected InputItem::Header");
    }

    let empty = InputItem::parse("X-Empty-Header;").unwrap();
    assert!(empty.is_header());
    if let InputItem::EmptyHeader { name } = empty {
        assert_eq!(name, "X-Empty-Header");
    } else {
        panic!("Expected InputItem::EmptyHeader");
    }

    let file_header = InputItem::parse("X-Token-File:@/tmp/token.txt").unwrap();
    assert!(file_header.is_header());
    assert!(file_header.requires_file_read());
    if let InputItem::HeaderFile { name, path } = file_header {
        assert_eq!(name, "X-Token-File");
        assert_eq!(path, PathBuf::from("/tmp/token.txt"));
    } else {
        panic!("Expected InputItem::HeaderFile");
    }
}

#[test]
fn test_parse_query_params() {
    let q = InputItem::parse("search==rust_lang").unwrap();
    assert!(q.is_query());
    assert!(!q.is_data());
    if let InputItem::QueryParam { name, value } = q {
        assert_eq!(name, "search");
        assert_eq!(value, "rust_lang");
    } else {
        panic!("Expected InputItem::QueryParam");
    }

    let q_file = InputItem::parse("q==@/tmp/query.txt").unwrap();
    assert!(q_file.is_query());
    assert!(q_file.requires_file_read());
    if let InputItem::QueryParamFile { name, path } = q_file {
        assert_eq!(name, "q");
        assert_eq!(path, PathBuf::from("/tmp/query.txt"));
    } else {
        panic!("Expected InputItem::QueryParamFile");
    }
}

#[test]
fn test_parse_data_fields() {
    let data = InputItem::parse("username=alex_dev").unwrap();
    assert!(data.is_data());
    assert!(!data.is_header());
    if let InputItem::DataField { key, value } = data {
        assert_eq!(key, "username");
        assert_eq!(value, "alex_dev");
    } else {
        panic!("Expected InputItem::DataField");
    }

    let data_file = InputItem::parse("bio=@/tmp/bio.txt").unwrap();
    assert!(data_file.is_data());
    assert!(data_file.requires_file_read());
    if let InputItem::DataFieldFile { key, path } = data_file {
        assert_eq!(key, "bio");
        assert_eq!(path, PathBuf::from("/tmp/bio.txt"));
    } else {
        panic!("Expected InputItem::DataFieldFile");
    }
}

#[test]
fn test_parse_json_fields() {
    let bool_item = InputItem::parse("active:=true").unwrap();
    assert!(bool_item.is_data());
    if let InputItem::JsonField { key, value } = bool_item {
        assert_eq!(key, "active");
        assert_eq!(value, json!(true));
    } else {
        panic!("Expected JsonField");
    }

    let int_item = InputItem::parse("count:=42").unwrap();
    if let InputItem::JsonField { key, value } = int_item {
        assert_eq!(key, "count");
        assert_eq!(value, json!(42));
    }

    let float_item = InputItem::parse("price:=19.99").unwrap();
    if let InputItem::JsonField { key, value } = float_item {
        assert_eq!(key, "price");
        assert_eq!(value, json!(19.99));
    }

    let array_item = InputItem::parse("tags:=[\"rust\",\"http\"]").unwrap();
    if let InputItem::JsonField { key, value } = array_item {
        assert_eq!(key, "tags");
        assert_eq!(value, json!(["rust", "http"]));
    }

    let object_item = InputItem::parse("meta:={\"admin\":false}").unwrap();
    if let InputItem::JsonField { key, value } = object_item {
        assert_eq!(key, "meta");
        assert_eq!(value, json!({"admin": false}));
    }

    let null_item = InputItem::parse("deleted_at:=null").unwrap();
    if let InputItem::JsonField { key, value } = null_item {
        assert_eq!(key, "deleted_at");
        assert_eq!(value, json!(null));
    }

    let json_file = InputItem::parse("config:=@/tmp/config.json").unwrap();
    assert!(json_file.requires_file_read());
    if let InputItem::JsonFieldFile { key, path } = json_file {
        assert_eq!(key, "config");
        assert_eq!(path, PathBuf::from("/tmp/config.json"));
    } else {
        panic!("Expected JsonFieldFile");
    }
}

#[test]
fn test_parse_file_upload() {
    let upload_simple = InputItem::parse("avatar@/tmp/avatar.png").unwrap();
    assert!(upload_simple.is_file_upload());
    assert!(upload_simple.is_data());
    if let InputItem::FileUpload {
        field,
        path,
        mime_type,
        filename,
    } = upload_simple
    {
        assert_eq!(field, "avatar");
        assert_eq!(path, PathBuf::from("/tmp/avatar.png"));
        assert_eq!(mime_type, None);
        assert_eq!(filename, None);
    } else {
        panic!("Expected FileUpload");
    }

    let upload_custom =
        InputItem::parse("doc@/tmp/file.bin;type=application/octet-stream;filename=custom.bin")
            .unwrap();
    assert!(upload_custom.is_file_upload());
    if let InputItem::FileUpload {
        field,
        path,
        mime_type,
        filename,
    } = upload_custom
    {
        assert_eq!(field, "doc");
        assert_eq!(path, PathBuf::from("/tmp/file.bin"));
        assert_eq!(mime_type, Some("application/octet-stream".to_string()));
        assert_eq!(filename, Some("custom.bin".to_string()));
    } else {
        panic!("Expected FileUpload with metadata");
    }
}
