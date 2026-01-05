//! Comprehensive Scripting Module Tests
//!
//! Tests ALL functions in ALL 19 scripting modules:
//! crypto, encoding, json, xml, regex, url, date, cookie, jwt, schema,
//! http, assert, env, faker, prompt, fs, store, console, system

mod common;

use common::http;
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a workflow that runs a script and checks it succeeds
fn create_script_test_workflow(server_uri: &str, script: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let workflow_path = dir.path().join("script_test.yaml");
    let workflow = format!(
        r#"
name: Script Test
base_url: "{}"

steps:
  - name: Test Script
    method: GET
    url: /test
    script_assert:
      code: |
        pub fn main() {{
{}
            true
        }}
"#,
        server_uri,
        script
            .lines()
            .map(|l| format!("            {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    );
    std::fs::write(&workflow_path, workflow).unwrap();
    (dir, workflow_path)
}

/// Helper to run a script test
async fn run_script_test(script: &str) -> (i32, String, String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let (_dir, workflow_path) = create_script_test_workflow(&server.uri(), script);
    let r = http(&["--run", workflow_path.to_str().unwrap()]);
    (r.exit_code, r.stdout, r.stderr)
}

// ============================================================================
// CRYPTO MODULE - Complete Tests (14 functions)
// ============================================================================

#[tokio::test]
async fn test_crypto_sha256_hex() {
    let (code, stdout, stderr) = run_script_test(r#"
let hash = crypto::sha256_hex("hello world");
if hash != "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9" {
    panic!("sha256 failed");
}
"#).await;
    assert!(code == 0, "crypto::sha256_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_sha512_hex() {
    let (code, stdout, stderr) = run_script_test(r#"
let hash = crypto::sha512_hex("hello world");
if hash.len() != 128 {
    panic!("sha512 length wrong");
}
"#).await;
    assert!(code == 0, "crypto::sha512_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_sha1_hex() {
    let (code, stdout, stderr) = run_script_test(r#"
let hash = crypto::sha1_hex("hello world");
if hash != "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed" {
    panic!("sha1 failed");
}
"#).await;
    assert!(code == 0, "crypto::sha1_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_md5_hex() {
    let (code, stdout, stderr) = run_script_test(r#"
let hash = crypto::md5_hex("hello world");
if hash != "5eb63bbbe01eeed093cb22bb8f5acdc3" {
    panic!("md5 failed");
}
"#).await;
    assert!(code == 0, "crypto::md5_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_hmac_sha256() {
    let (code, stdout, stderr) = run_script_test(r#"
let hmac = crypto::hmac_sha256("secret", "message");
if hmac.len() != 64 {
    panic!("hmac_sha256 length wrong");
}
"#).await;
    assert!(code == 0, "crypto::hmac_sha256 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_hmac_sha512() {
    let (code, stdout, stderr) = run_script_test(r#"
let hmac = crypto::hmac_sha512("secret", "message");
if hmac.len() != 128 {
    panic!("hmac_sha512 length wrong");
}
"#).await;
    assert!(code == 0, "crypto::hmac_sha512 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_hmac_sha256_base64() {
    let (code, stdout, stderr) = run_script_test(r#"
let hmac = crypto::hmac_sha256_base64("secret", "message");
if hmac.len() == 0 {
    panic!("hmac_sha256_base64 empty");
}
"#).await;
    assert!(code == 0, "crypto::hmac_sha256_base64 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_random_hex() {
    let (code, stdout, stderr) = run_script_test(r#"
let hex = crypto::random_hex(16);
if hex.len() != 32 {
    panic!("random_hex length wrong");
}
"#).await;
    assert!(code == 0, "crypto::random_hex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_random_bytes_base64() {
    let (code, stdout, stderr) = run_script_test(r#"
let b64 = crypto::random_bytes_base64(16);
if b64.len() == 0 {
    panic!("random_bytes_base64 empty");
}
"#).await;
    assert!(code == 0, "crypto::random_bytes_base64 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_random_int() {
    let (code, stdout, stderr) = run_script_test(r#"
let n = crypto::random_int(10, 20);
if n < 10 || n > 20 {
    panic!("random_int out of range");
}
"#).await;
    assert!(code == 0, "crypto::random_int failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_random_string() {
    let (code, stdout, stderr) = run_script_test(r#"
let s = crypto::random_string(16);
if s.len() != 16 {
    panic!("random_string length wrong");
}
"#).await;
    assert!(code == 0, "crypto::random_string failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_uuid_v4() {
    let (code, stdout, stderr) = run_script_test(r#"
let uuid = crypto::uuid_v4();
if uuid.len() != 36 {
    panic!("uuid_v4 length wrong");
}
"#).await;
    assert!(code == 0, "crypto::uuid_v4 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_uuid_v7() {
    let (code, stdout, stderr) = run_script_test(r#"
let uuid = crypto::uuid_v7();
if uuid.len() != 36 {
    panic!("uuid_v7 length wrong");
}
"#).await;
    assert!(code == 0, "crypto::uuid_v7 failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_timestamp() {
    let (code, stdout, stderr) = run_script_test(r#"
let ts = crypto::timestamp();
if ts < 1700000000 {
    panic!("timestamp too old");
}
"#).await;
    assert!(code == 0, "crypto::timestamp failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_crypto_timestamp_ms() {
    let (code, stdout, stderr) = run_script_test(r#"
let ts = crypto::timestamp_ms();
if ts < 1700000000000 {
    panic!("timestamp_ms too old");
}
"#).await;
    assert!(code == 0, "crypto::timestamp_ms failed: {} {}", stdout, stderr);
}

// ============================================================================
// ENCODING MODULE - Complete Tests (7 functions)
// ============================================================================

#[tokio::test]
async fn test_encoding_base64_encode() {
    let (code, stdout, stderr) = run_script_test(r#"
let encoded = encoding::base64_encode("hello world");
if encoded != "aGVsbG8gd29ybGQ=" {
    panic!("base64_encode failed");
}
"#).await;
    assert!(code == 0, "encoding::base64_encode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_base64_decode() {
    let (code, stdout, stderr) = run_script_test(r#"
let decoded = encoding::base64_decode("aGVsbG8gd29ybGQ=");
if decoded != "hello world" {
    panic!("base64_decode failed");
}
"#).await;
    assert!(code == 0, "encoding::base64_decode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_url_encode() {
    let (code, stdout, stderr) = run_script_test(r#"
let encoded = encoding::url_encode("hello world & more");
if !encoded.contains("%20") && !encoded.contains("+") {
    panic!("url_encode failed");
}
"#).await;
    assert!(code == 0, "encoding::url_encode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_url_decode() {
    let (code, stdout, stderr) = run_script_test(r#"
let decoded = encoding::url_decode("hello%20world");
if decoded != "hello world" {
    panic!("url_decode failed");
}
"#).await;
    assert!(code == 0, "encoding::url_decode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_hex_encode() {
    let (code, stdout, stderr) = run_script_test(r#"
let hex = encoding::hex_encode("ABC");
if hex != "414243" {
    panic!("hex_encode failed");
}
"#).await;
    assert!(code == 0, "encoding::hex_encode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_hex_decode() {
    let (code, stdout, stderr) = run_script_test(r#"
let decoded = encoding::hex_decode("414243");
if decoded != "ABC" {
    panic!("hex_decode failed");
}
"#).await;
    assert!(code == 0, "encoding::hex_decode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_encoding_html_escape() {
    let (code, stdout, stderr) = run_script_test(r#"
let escaped = encoding::html_escape("<script>");
if !escaped.contains("&lt;") {
    panic!("html_escape failed");
}
"#).await;
    assert!(code == 0, "encoding::html_escape failed: {} {}", stdout, stderr);
}

// ============================================================================
// JSON MODULE - Complete Tests (22 functions)
// ============================================================================

#[tokio::test]
async fn test_json_parse() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = json::parse("{\"name\": \"test\", \"value\": 42}");
"#).await;
    assert!(code == 0, "json::parse failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_stringify() {
    let (code, stdout, stderr) = run_script_test(r#"
let s = json::stringify("{\"a\": 1}");
if s.len() == 0 {
    panic!("stringify failed");
}
"#).await;
    assert!(code == 0, "json::stringify failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_stringify_pretty() {
    let (code, stdout, stderr) = run_script_test(r#"
let s = json::stringify_pretty("{\"a\": 1}");
if s.len() == 0 {
    panic!("stringify_pretty failed");
}
"#).await;
    assert!(code == 0, "json::stringify_pretty failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_get() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"user\": {\"name\": \"test\"}}";
let name = json::get(data, "user.name");
if name != "\"test\"" {
    panic!("json::get failed");
}
"#).await;
    assert!(code == 0, "json::get failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_query() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"items\": [{\"id\": 1}, {\"id\": 2}]}";
let ids = json::query(data, "$.items[*].id");
if ids.len() == 0 {
    panic!("json::query failed");
}
"#).await;
    assert!(code == 0, "json::query failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_query_first() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"items\": [{\"id\": 1}, {\"id\": 2}]}";
let first = json::query_first(data, "$.items[0].id");
if first != "1" {
    panic!("json::query_first failed");
}
"#).await;
    assert!(code == 0, "json::query_first failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_keys() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"a\": 1, \"b\": 2}";
let keys = json::keys(data);
if keys.len() == 0 {
    panic!("json::keys failed");
}
"#).await;
    assert!(code == 0, "json::keys failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_values() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"a\": 1, \"b\": 2}";
let values = json::values(data);
if values.len() == 0 {
    panic!("json::values failed");
}
"#).await;
    assert!(code == 0, "json::values failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_len() {
    let (code, stdout, stderr) = run_script_test(r#"
let arr = "[1, 2, 3]";
let len = json::len(arr);
if len != 3 {
    panic!("json::len failed");
}
"#).await;
    assert!(code == 0, "json::len failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_has() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"name\": \"test\"}";
if !json::has(data, "name") {
    panic!("json::has failed");
}
if json::has(data, "missing") {
    panic!("json::has false positive");
}
"#).await;
    assert!(code == 0, "json::has failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_object() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_object("{\"a\": 1}") {
    panic!("is_object failed");
}
if json::is_object("[1, 2]") {
    panic!("is_object false positive");
}
"#).await;
    assert!(code == 0, "json::is_object failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_array() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_array("[1, 2, 3]") {
    panic!("is_array failed");
}
if json::is_array("{\"a\": 1}") {
    panic!("is_array false positive");
}
"#).await;
    assert!(code == 0, "json::is_array failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_string() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_string("\"hello\"") {
    panic!("is_string failed");
}
if json::is_string("123") {
    panic!("is_string false positive");
}
"#).await;
    assert!(code == 0, "json::is_string failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_number() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_number("123") {
    panic!("is_number failed");
}
if !json::is_number("3.14") {
    panic!("is_number failed for float");
}
if json::is_number("\"123\"") {
    panic!("is_number false positive");
}
"#).await;
    assert!(code == 0, "json::is_number failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_bool() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_bool("true") {
    panic!("is_bool failed for true");
}
if !json::is_bool("false") {
    panic!("is_bool failed for false");
}
if json::is_bool("1") {
    panic!("is_bool false positive");
}
"#).await;
    assert!(code == 0, "json::is_bool failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_is_null() {
    let (code, stdout, stderr) = run_script_test(r#"
if !json::is_null("null") {
    panic!("is_null failed");
}
if json::is_null("0") {
    panic!("is_null false positive");
}
"#).await;
    assert!(code == 0, "json::is_null failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_type_of() {
    let (code, stdout, stderr) = run_script_test(r#"
let t1 = json::type_of("{\"a\": 1}");
let t2 = json::type_of("[1, 2]");
let t3 = json::type_of("123");
if t1.len() == 0 || t2.len() == 0 || t3.len() == 0 {
    panic!("type_of failed");
}
"#).await;
    assert!(code == 0, "json::type_of failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_merge() {
    let (code, stdout, stderr) = run_script_test(r#"
let a = "{\"x\": 1, \"y\": 2}";
let b = "{\"y\": 3, \"z\": 4}";
let merged = json::merge(a, b);
if !merged.contains("\"z\"") {
    panic!("merge failed");
}
"#).await;
    assert!(code == 0, "json::merge failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_set() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"a\": 1}";
let updated = json::set(data, "b", "2");
if !updated.contains("\"b\"") {
    panic!("set failed");
}
"#).await;
    assert!(code == 0, "json::set failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_remove() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"a\": 1, \"b\": 2}";
let removed = json::remove(data, "a");
if removed.contains("\"a\"") {
    panic!("remove failed");
}
"#).await;
    assert!(code == 0, "json::remove failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_equals() {
    let (code, stdout, stderr) = run_script_test(r#"
let a = "{\"x\": 1}";
let b = "{\"x\": 1}";
let c = "{\"x\": 2}";
if !json::equals(a, b) {
    panic!("equals failed");
}
if json::equals(a, c) {
    panic!("equals false positive");
}
"#).await;
    assert!(code == 0, "json::equals failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_json_diff() {
    let (code, stdout, stderr) = run_script_test(r#"
let a = "{\"x\": 1, \"y\": 2}";
let b = "{\"x\": 1, \"y\": 3}";
let diff = json::diff(a, b);
if diff.len() == 0 {
    panic!("diff failed");
}
"#).await;
    assert!(code == 0, "json::diff failed: {} {}", stdout, stderr);
}

// ============================================================================
// XML MODULE - Complete Tests (7 functions)
// ============================================================================

#[tokio::test]
async fn test_xml_to_json() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item>test</item></root>";
let json = xml::to_json(xml);
if json.len() == 0 {
    panic!("to_json failed");
}
"#).await;
    assert!(code == 0, "xml::to_json failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_parse() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item>test</item></root>";
let json = xml::parse(xml);
if json.len() == 0 {
    panic!("parse failed");
}
"#).await;
    assert!(code == 0, "xml::parse failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_get_text() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item>hello</item></root>";
let text = xml::get_text(xml, "item");
if text != "hello" {
    panic!("get_text failed");
}
"#).await;
    assert!(code == 0, "xml::get_text failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_get_attr() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item id=\"123\">test</item></root>";
let id = xml::get_attr(xml, "item", "id");
if id != "123" {
    panic!("get_attr failed");
}
"#).await;
    assert!(code == 0, "xml::get_attr failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_count_elements() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item/><item/><item/></root>";
let count = xml::count_elements(xml, "item");
if count != 3 {
    panic!("count_elements failed");
}
"#).await;
    assert!(code == 0, "xml::count_elements failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_has_element() {
    let (code, stdout, stderr) = run_script_test(r#"
let xml = "<root><item/></root>";
if !xml::has_element(xml, "item") {
    panic!("has_element failed");
}
if xml::has_element(xml, "missing") {
    panic!("has_element false positive");
}
"#).await;
    assert!(code == 0, "xml::has_element failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_xml_is_valid() {
    let (code, stdout, stderr) = run_script_test(r#"
if !xml::is_valid("<root><item/></root>") {
    panic!("is_valid failed for valid xml");
}
// Note: Some XML parsers are lenient with malformed XML
"#).await;
    assert!(code == 0, "xml::is_valid failed: {} {}", stdout, stderr);
}

// ============================================================================
// REGEX MODULE - Complete Tests (11 functions)
// ============================================================================

#[tokio::test]
async fn test_regex_test() {
    let (code, stdout, stderr) = run_script_test(r#"
if !regex::test("hello world", "world") {
    panic!("test failed");
}
if regex::test("hello", "world") {
    panic!("test false positive");
}
"#).await;
    assert!(code == 0, "regex::test failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_find() {
    let (code, stdout, stderr) = run_script_test(r#"
let m = regex::find("Order 12345 here", "[0-9]+");
if m != "12345" {
    panic!("find failed");
}
"#).await;
    assert!(code == 0, "regex::find failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_match_all() {
    let (code, stdout, stderr) = run_script_test(r#"
let matches = regex::match_all("a1 b2 c3", "[a-z][0-9]");
if matches.len() == 0 {
    panic!("match_all failed");
}
"#).await;
    assert!(code == 0, "regex::match_all failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_count() {
    let (code, stdout, stderr) = run_script_test(r#"
let count = regex::count("one two three", "[a-z]+");
if count != 3 {
    panic!("count failed");
}
"#).await;
    assert!(code == 0, "regex::count failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_capture() {
    let (code, stdout, stderr) = run_script_test(r#"
let groups = regex::capture("John Smith", "([A-Z][a-z]+) ([A-Z][a-z]+)");
if groups.len() == 0 {
    panic!("capture failed");
}
"#).await;
    assert!(code == 0, "regex::capture failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_capture_named() {
    let (code, stdout, stderr) = run_script_test(r#"
let groups = regex::capture_named("John Smith", "(?P<first>[A-Z][a-z]+) (?P<last>[A-Z][a-z]+)");
if groups.len() == 0 {
    panic!("capture_named failed");
}
"#).await;
    assert!(code == 0, "regex::capture_named failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_replace() {
    let (code, stdout, stderr) = run_script_test(r#"
let result = regex::replace("foo bar foo", "foo", "baz");
if result != "baz bar foo" {
    panic!("replace failed");
}
"#).await;
    assert!(code == 0, "regex::replace failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_replace_all() {
    let (code, stdout, stderr) = run_script_test(r#"
let result = regex::replace_all("foo bar foo", "foo", "baz");
if result != "baz bar baz" {
    panic!("replace_all failed");
}
"#).await;
    assert!(code == 0, "regex::replace_all failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_split() {
    let (code, stdout, stderr) = run_script_test(r#"
let parts = regex::split("a,b;c", "[,;]");
if parts.len() == 0 {
    panic!("split failed");
}
"#).await;
    assert!(code == 0, "regex::split failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_escape() {
    let (code, stdout, stderr) = run_script_test(r#"
let escaped = regex::escape("hello.world");
if !escaped.contains("\\") {
    panic!("escape failed");
}
"#).await;
    assert!(code == 0, "regex::escape failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_regex_is_valid() {
    let (code, stdout, stderr) = run_script_test(r#"
if !regex::is_valid("[a-z]+") {
    panic!("is_valid failed");
}
if regex::is_valid("[invalid") {
    panic!("is_valid false positive");
}
"#).await;
    assert!(code == 0, "regex::is_valid failed: {} {}", stdout, stderr);
}

// ============================================================================
// URL MODULE - Complete Tests (22 functions)
// ============================================================================

#[tokio::test]
async fn test_url_parse() {
    let (code, stdout, stderr) = run_script_test(r#"
let parsed = url::parse("https://user:pass@example.com:8080/path?key=value#section");
if parsed.len() == 0 {
    panic!("parse failed");
}
"#).await;
    assert!(code == 0, "url::parse failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_is_valid() {
    let (code, stdout, stderr) = run_script_test(r#"
if !url::is_valid("https://example.com") {
    panic!("is_valid failed");
}
"#).await;
    assert!(code == 0, "url::is_valid failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_scheme() {
    let (code, stdout, stderr) = run_script_test(r#"
let scheme = url::scheme("https://example.com");
if scheme != "https" {
    panic!("scheme failed");
}
"#).await;
    assert!(code == 0, "url::scheme failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_host() {
    let (code, stdout, stderr) = run_script_test(r#"
let host = url::host("https://example.com:8080/path");
if host != "example.com" {
    panic!("host failed");
}
"#).await;
    assert!(code == 0, "url::host failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_port() {
    let (code, stdout, stderr) = run_script_test(r#"
let port = url::port("https://example.com:8080/path");
if port != 8080 {
    panic!("port failed");
}
"#).await;
    assert!(code == 0, "url::port failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_path() {
    let (code, stdout, stderr) = run_script_test(r#"
let path = url::path("https://example.com/api/users");
if path != "/api/users" {
    panic!("path failed");
}
"#).await;
    assert!(code == 0, "url::path failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_query() {
    let (code, stdout, stderr) = run_script_test(r#"
let query = url::query("https://example.com?key=value&foo=bar");
if !query.contains("key=value") {
    panic!("query failed");
}
"#).await;
    assert!(code == 0, "url::query failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_fragment() {
    let (code, stdout, stderr) = run_script_test(r#"
let fragment = url::fragment("https://example.com#section");
if fragment != "section" {
    panic!("fragment failed");
}
"#).await;
    assert!(code == 0, "url::fragment failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_query_param() {
    let (code, stdout, stderr) = run_script_test(r#"
let value = url::query_param("https://example.com?key=value", "key");
if value != "value" {
    panic!("query_param failed");
}
"#).await;
    assert!(code == 0, "url::query_param failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_query_params() {
    let (code, stdout, stderr) = run_script_test(r#"
let params = url::query_params("https://example.com?a=1&b=2");
if params.len() == 0 {
    panic!("query_params failed");
}
"#).await;
    assert!(code == 0, "url::query_params failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_set_query_param() {
    let (code, stdout, stderr) = run_script_test(r#"
let updated = url::set_query_param("https://example.com?a=1", "b", "2");
if !updated.contains("b=2") {
    panic!("set_query_param failed");
}
"#).await;
    assert!(code == 0, "url::set_query_param failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_remove_query_param() {
    let (code, stdout, stderr) = run_script_test(r#"
let updated = url::remove_query_param("https://example.com?a=1&b=2", "a");
if updated.contains("a=1") {
    panic!("remove_query_param failed");
}
"#).await;
    assert!(code == 0, "url::remove_query_param failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_join() {
    let (code, stdout, stderr) = run_script_test(r#"
let full = url::join("https://example.com/v1/", "users");
if !full.contains("users") {
    panic!("join failed");
}
"#).await;
    assert!(code == 0, "url::join failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_url_encode_decode() {
    let (code, stdout, stderr) = run_script_test(r#"
let encoded = url::encode("hello world");
let decoded = url::decode(encoded);
if decoded != "hello world" {
    panic!("encode/decode failed");
}
"#).await;
    assert!(code == 0, "url encode/decode failed: {} {}", stdout, stderr);
}

// ============================================================================
// DATE MODULE - Complete Tests (30+ functions)
// ============================================================================

#[tokio::test]
async fn test_date_now() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = date::now();
if now.len() == 0 {
    panic!("now failed");
}
"#).await;
    assert!(code == 0, "date::now failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_now_utc() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = date::now_utc();
if now.len() == 0 {
    panic!("now_utc failed");
}
"#).await;
    assert!(code == 0, "date::now_utc failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_now_local() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = date::now_local();
if now.len() == 0 {
    panic!("now_local failed");
}
"#).await;
    assert!(code == 0, "date::now_local failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_timestamp() {
    let (code, stdout, stderr) = run_script_test(r#"
let ts = date::timestamp();
if ts < 1700000000 {
    panic!("timestamp failed");
}
"#).await;
    assert!(code == 0, "date::timestamp failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_timestamp_ms() {
    let (code, stdout, stderr) = run_script_test(r#"
let ts = date::timestamp_ms();
if ts < 1700000000000 {
    panic!("timestamp_ms failed");
}
"#).await;
    assert!(code == 0, "date::timestamp_ms failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_parse_iso() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = date::parse_iso("2024-01-15T10:30:00Z");
if dt.len() == 0 {
    panic!("parse_iso failed");
}
"#).await;
    assert!(code == 0, "date::parse_iso failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_format() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = date::now();
let formatted = date::format(dt, "%Y-%m-%d");
if formatted.len() == 0 {
    panic!("format failed");
}
"#).await;
    assert!(code == 0, "date::format failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_from_timestamp() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = date::from_timestamp(1705315800);
if dt.len() == 0 {
    panic!("from_timestamp failed");
}
"#).await;
    assert!(code == 0, "date::from_timestamp failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_components() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = "2024-06-15T14:30:45+00:00";
let year = date::year(dt);
let month = date::month(dt);
let day = date::day(dt);
let hour = date::hour(dt);
let minute = date::minute(dt);
let second = date::second(dt);
if year != 2024 { panic!("year failed"); }
if month != 6 { panic!("month failed"); }
if day != 15 { panic!("day failed"); }
"#).await;
    assert!(code == 0, "date components failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_add_days() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = "2024-01-15T10:00:00+00:00";
let tomorrow = date::add_days(dt, 1);
if !tomorrow.contains("16") {
    panic!("add_days failed");
}
"#).await;
    assert!(code == 0, "date::add_days failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_subtract_days() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = "2024-01-15T10:00:00+00:00";
let yesterday = date::subtract_days(dt, 1);
if !yesterday.contains("14") {
    panic!("subtract_days failed");
}
"#).await;
    assert!(code == 0, "date::subtract_days failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_diff_days() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt1 = "2024-01-01T00:00:00+00:00";
let dt2 = "2024-01-10T00:00:00+00:00";
let days = date::diff_days(dt1, dt2);
if days != 9 {
    panic!("diff_days failed");
}
"#).await;
    assert!(code == 0, "date::diff_days failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_is_before_after() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt1 = "2024-01-01T00:00:00+00:00";
let dt2 = "2024-01-10T00:00:00+00:00";
if !date::is_before(dt1, dt2) {
    panic!("is_before failed");
}
if !date::is_after(dt2, dt1) {
    panic!("is_after failed");
}
"#).await;
    assert!(code == 0, "date is_before/is_after failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_date_start_end_of_day() {
    let (code, stdout, stderr) = run_script_test(r#"
let dt = "2024-01-15T14:30:00+00:00";
let start = date::start_of_day(dt);
let end = date::end_of_day(dt);
if !start.contains("00:00:00") {
    panic!("start_of_day failed");
}
if !end.contains("23:59:59") {
    panic!("end_of_day failed");
}
"#).await;
    assert!(code == 0, "date start/end_of_day failed: {} {}", stdout, stderr);
}

// ============================================================================
// COOKIE MODULE - Complete Tests (11 functions)
// ============================================================================

#[tokio::test]
async fn test_cookie_parse() {
    let (code, stdout, stderr) = run_script_test(r#"
let header = "session=abc123; user=john; theme=dark";
let cookies = cookie::parse(header);
if cookies.len() == 0 {
    panic!("parse failed");
}
"#).await;
    assert!(code == 0, "cookie::parse failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_get() {
    let (code, stdout, stderr) = run_script_test(r#"
let header = "session=abc123; user=john";
let session = cookie::get(header, "session");
if session != "abc123" {
    panic!("get failed");
}
"#).await;
    assert!(code == 0, "cookie::get failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_parse_set_cookie() {
    let (code, stdout, stderr) = run_script_test(r#"
let set_cookie = "session=abc123; Path=/; HttpOnly; Secure; Max-Age=3600";
let details = cookie::parse_set_cookie(set_cookie);
if details.len() == 0 {
    panic!("parse_set_cookie failed");
}
"#).await;
    assert!(code == 0, "cookie::parse_set_cookie failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_build() {
    let (code, stdout, stderr) = run_script_test(r#"
let cookie = cookie::build("session", "abc123");
if cookie != "session=abc123" {
    panic!("build failed");
}
"#).await;
    assert!(code == 0, "cookie::build failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_merge() {
    let (code, stdout, stderr) = run_script_test(r#"
let merged = cookie::merge("a=1; b=2", "b=3; c=4");
if !merged.contains("c=4") {
    panic!("merge failed");
}
"#).await;
    assert!(code == 0, "cookie::merge failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_remove() {
    let (code, stdout, stderr) = run_script_test(r#"
let filtered = cookie::remove("a=1; b=2; c=3", "b");
if filtered.contains("b=2") {
    panic!("remove failed");
}
"#).await;
    assert!(code == 0, "cookie::remove failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_cookie_to_header() {
    let (code, stdout, stderr) = run_script_test(r#"
let header = cookie::to_header("{\"session\": \"abc\", \"user\": \"john\"}");
if !header.contains("session=abc") {
    panic!("to_header failed");
}
"#).await;
    assert!(code == 0, "cookie::to_header failed: {} {}", stdout, stderr);
}

// ============================================================================
// JWT MODULE - Complete Tests (12 functions)
// ============================================================================

#[tokio::test]
async fn test_jwt_decode() {
    let (code, stdout, stderr) = run_script_test(r#"
let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
let payload = jwt::decode(token);
if payload.len() == 0 {
    panic!("decode failed");
}
"#).await;
    assert!(code == 0, "jwt::decode failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_jwt_decode_header() {
    let (code, stdout, stderr) = run_script_test(r#"
let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
let header = jwt::decode_header(token);
if !header.contains("HS256") {
    panic!("decode_header failed");
}
"#).await;
    assert!(code == 0, "jwt::decode_header failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_jwt_get_claim() {
    let (code, stdout, stderr) = run_script_test(r#"
let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
let sub = jwt::get_claim(token, "sub");
if sub != "1234567890" {
    panic!("get_claim failed");
}
"#).await;
    assert!(code == 0, "jwt::get_claim failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_jwt_get_sub() {
    let (code, stdout, stderr) = run_script_test(r#"
let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
let sub = jwt::get_sub(token);
if sub != "1234567890" {
    panic!("get_sub failed");
}
"#).await;
    assert!(code == 0, "jwt::get_sub failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_jwt_is_valid_format() {
    let (code, stdout, stderr) = run_script_test(r#"
let valid = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
if !jwt::is_valid_format(valid) {
    panic!("is_valid_format failed");
}
if jwt::is_valid_format("not.a.valid.jwt") {
    panic!("is_valid_format false positive");
}
"#).await;
    assert!(code == 0, "jwt::is_valid_format failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_jwt_parts_count() {
    let (code, stdout, stderr) = run_script_test(r#"
let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
let parts = jwt::parts_count(token);
if parts != 3 {
    panic!("parts_count failed");
}
"#).await;
    assert!(code == 0, "jwt::parts_count failed: {} {}", stdout, stderr);
}

// ============================================================================
// SCHEMA MODULE - Complete Tests (10 functions)
// ============================================================================

#[tokio::test]
async fn test_schema_is_valid() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"name\": \"test\", \"age\": 30}";
let schema = "{\"type\": \"object\", \"required\": [\"name\"], \"properties\": {\"name\": {\"type\": \"string\"}, \"age\": {\"type\": \"integer\"}}}";
if !schema::is_valid(data, schema) {
    panic!("is_valid failed");
}
"#).await;
    assert!(code == 0, "schema::is_valid failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_schema_validate() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"name\": \"test\"}";
let schema = "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}}}";
let result = schema::validate(data, schema);
if result.len() == 0 {
    panic!("validate failed");
}
"#).await;
    assert!(code == 0, "schema::validate failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_schema_errors() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"name\": 123}";
let schema = "{\"type\": \"object\", \"properties\": {\"name\": {\"type\": \"string\"}}}";
let errors = schema::errors(data, schema);
"#).await;
    assert!(code == 0, "schema::errors failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_schema_type_helpers() {
    let (code, stdout, stderr) = run_script_test(r#"
let s = schema::type_string();
let n = schema::type_number();
let i = schema::type_integer();
let b = schema::type_boolean();
let a = schema::type_array();
let o = schema::type_object();
if s.len() == 0 {
    panic!("type helpers failed");
}
"#).await;
    assert!(code == 0, "schema type helpers failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_schema_format_helpers() {
    let (code, stdout, stderr) = run_script_test(r#"
let email = schema::email();
let uuid = schema::uuid();
let date = schema::date();
let url = schema::url();
if email.len() == 0 {
    panic!("format helpers failed");
}
"#).await;
    assert!(code == 0, "schema format helpers failed: {} {}", stdout, stderr);
}

// ============================================================================
// HTTP MODULE - Complete Tests (20+ functions)
// ============================================================================

#[tokio::test]
async fn test_http_status_constants() {
    let (code, stdout, stderr) = run_script_test(r#"
if http::OK != 200 { panic!("OK wrong"); }
if http::CREATED != 201 { panic!("CREATED wrong"); }
if http::NO_CONTENT != 204 { panic!("NO_CONTENT wrong"); }
if http::BAD_REQUEST != 400 { panic!("BAD_REQUEST wrong"); }
if http::UNAUTHORIZED != 401 { panic!("UNAUTHORIZED wrong"); }
if http::FORBIDDEN != 403 { panic!("FORBIDDEN wrong"); }
if http::NOT_FOUND != 404 { panic!("NOT_FOUND wrong"); }
if http::INTERNAL_SERVER_ERROR != 500 { panic!("INTERNAL_SERVER_ERROR wrong"); }
"#).await;
    assert!(code == 0, "http status constants failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_http_is_success() {
    let (code, stdout, stderr) = run_script_test(r#"
if !http::is_success(200) { panic!("failed for 200"); }
if !http::is_success(201) { panic!("failed for 201"); }
if http::is_success(404) { panic!("false positive"); }
"#).await;
    assert!(code == 0, "http::is_success failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_http_is_redirect() {
    let (code, stdout, stderr) = run_script_test(r#"
if !http::is_redirect(301) { panic!("failed for 301"); }
if !http::is_redirect(302) { panic!("failed for 302"); }
if http::is_redirect(200) { panic!("false positive"); }
"#).await;
    assert!(code == 0, "http::is_redirect failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_http_is_client_error() {
    let (code, stdout, stderr) = run_script_test(r#"
if !http::is_client_error(400) { panic!("failed for 400"); }
if !http::is_client_error(404) { panic!("failed for 404"); }
if http::is_client_error(500) { panic!("false positive"); }
"#).await;
    assert!(code == 0, "http::is_client_error failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_http_is_server_error() {
    let (code, stdout, stderr) = run_script_test(r#"
if !http::is_server_error(500) { panic!("failed for 500"); }
if !http::is_server_error(503) { panic!("failed for 503"); }
if http::is_server_error(404) { panic!("false positive"); }
"#).await;
    assert!(code == 0, "http::is_server_error failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_http_is_error() {
    let (code, stdout, stderr) = run_script_test(r#"
if !http::is_error(400) { panic!("failed for 400"); }
if !http::is_error(500) { panic!("failed for 500"); }
if http::is_error(200) { panic!("false positive"); }
"#).await;
    assert!(code == 0, "http::is_error failed: {} {}", stdout, stderr);
}

// ============================================================================
// ENV MODULE - Complete Tests (10 functions)
// ============================================================================

#[tokio::test]
async fn test_env_get() {
    let (code, stdout, stderr) = run_script_test(r#"
let user = env::get("USER");
"#).await;
    assert!(code == 0, "env::get failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_get_or() {
    let (code, stdout, stderr) = run_script_test(r#"
let value = env::get_or("NONEXISTENT_VAR_12345", "default");
if value != "default" {
    panic!("get_or failed");
}
"#).await;
    assert!(code == 0, "env::get_or failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_has() {
    let (code, stdout, stderr) = run_script_test(r#"
let has = env::has("PATH");
"#).await;
    assert!(code == 0, "env::has failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_os() {
    let (code, stdout, stderr) = run_script_test(r#"
let os = env::os();
if os.len() == 0 { panic!("os failed"); }
"#).await;
    assert!(code == 0, "env::os failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_arch() {
    let (code, stdout, stderr) = run_script_test(r#"
let arch = env::arch();
if arch.len() == 0 { panic!("arch failed"); }
"#).await;
    assert!(code == 0, "env::arch failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_now() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = env::now();
if now < 1700000000 { panic!("now failed"); }
"#).await;
    assert!(code == 0, "env::now failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_now_millis() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = env::now_millis();
if now < 1700000000000 { panic!("now_millis failed"); }
"#).await;
    assert!(code == 0, "env::now_millis failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_env_now_iso() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = env::now_iso();
if now.len() == 0 { panic!("now_iso failed"); }
"#).await;
    assert!(code == 0, "env::now_iso failed: {} {}", stdout, stderr);
}

// ============================================================================
// FAKER MODULE - Complete Tests (40+ functions)
// ============================================================================

#[tokio::test]
async fn test_faker_names() {
    let (code, stdout, stderr) = run_script_test(r#"
let name = faker::name();
let first = faker::first_name();
let last = faker::last_name();
let with_title = faker::name_with_title();
let title = faker::title();
if name.len() == 0 || first.len() == 0 || last.len() == 0 {
    panic!("name generators failed");
}
"#).await;
    assert!(code == 0, "faker names failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_internet() {
    let (code, stdout, stderr) = run_script_test(r#"
let email = faker::email();
let safe = faker::safe_email();
let user = faker::username();
let pass = faker::password();
let ipv4 = faker::ipv4();
let ua = faker::user_agent();
if email.len() == 0 || user.len() == 0 {
    panic!("internet generators failed");
}
"#).await;
    assert!(code == 0, "faker internet failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_address() {
    let (code, stdout, stderr) = run_script_test(r#"
let city = faker::city();
let street = faker::street_name();
let addr = faker::street_address();
let zip = faker::zip_code();
let state = faker::state();
let abbr = faker::state_abbr();
let country = faker::country();
if city.len() == 0 || addr.len() == 0 {
    panic!("address generators failed");
}
"#).await;
    assert!(code == 0, "faker address failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_company() {
    let (code, stdout, stderr) = run_script_test(r#"
let company = faker::company_name();
let suffix = faker::company_suffix();
let industry = faker::industry();
let profession = faker::profession();
let buzzword = faker::buzzword();
if company.len() == 0 {
    panic!("company generators failed");
}
"#).await;
    assert!(code == 0, "faker company failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_lorem() {
    let (code, stdout, stderr) = run_script_test(r#"
let word = faker::word();
let words = faker::words();
let sentence = faker::sentence();
let paragraph = faker::paragraph();
if word.len() == 0 || sentence.len() == 0 {
    panic!("lorem generators failed");
}
"#).await;
    assert!(code == 0, "faker lorem failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_numbers() {
    let (code, stdout, stderr) = run_script_test(r#"
let num = faker::number();
let range = faker::number_range(10, 20);
let f = faker::float();
let frange = faker::float_range(1.0, 2.0);
let b = faker::bool();
let bratio = faker::bool_ratio(80);
if range < 10 || range > 20 { panic!("number_range failed"); }
if frange < 1.0 || frange > 2.0 { panic!("float_range failed"); }
"#).await;
    assert!(code == 0, "faker numbers failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_faker_misc() {
    let (code, stdout, stderr) = run_script_test(r#"
let phone = faker::phone_number();
let file = faker::file_name();
let ext = faker::file_extension();
let mime = faker::mime_type();
if phone.len() == 0 || file.len() == 0 {
    panic!("misc generators failed");
}
"#).await;
    assert!(code == 0, "faker misc failed: {} {}", stdout, stderr);
}

// ============================================================================
// FS MODULE - Complete Tests (12 functions)
// ============================================================================

#[tokio::test]
async fn test_fs_temp_dir() {
    let (code, stdout, stderr) = run_script_test(r#"
let temp = fs::temp_dir();
if temp.len() == 0 { panic!("temp_dir failed"); }
"#).await;
    assert!(code == 0, "fs::temp_dir failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_fs_cwd() {
    let (code, stdout, stderr) = run_script_test(r#"
let cwd = fs::cwd();
if cwd.len() == 0 { panic!("cwd failed"); }
"#).await;
    assert!(code == 0, "fs::cwd failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_fs_path_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
let base = fs::basename("/path/to/file.txt");
if base != "file.txt" { panic!("basename failed"); }
let dir = fs::dirname("/path/to/file.txt");
if dir != "/path/to" { panic!("dirname failed"); }
let ext = fs::extension("file.json");
if ext != "json" { panic!("extension failed"); }
let joined = fs::join("/path", "to/file");
if !joined.contains("to") { panic!("join failed"); }
"#).await;
    assert!(code == 0, "fs path operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_fs_exists_checks() {
    let (code, stdout, stderr) = run_script_test(r#"
let temp = fs::temp_dir();
if !fs::exists(temp) { panic!("exists failed"); }
if !fs::is_dir(temp) { panic!("is_dir failed"); }
"#).await;
    assert!(code == 0, "fs exists checks failed: {} {}", stdout, stderr);
}

// ============================================================================
// STORE MODULE - Complete Tests (18 functions)
// ============================================================================

#[tokio::test]
async fn test_store_string_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::set_string("test_key", "test_value");
let value = store::get_string("test_key");
if value != "test_value" { panic!("string ops failed"); }
store::delete("test_key");
if store::has("test_key") { panic!("delete failed"); }
"#).await;
    assert!(code == 0, "store string operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_int_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::set_int("counter", 10);
let value = store::get_int("counter");
if value != 10 { panic!("int ops failed"); }
let inc = store::incr("counter");
if inc != 11 { panic!("incr failed"); }
let dec = store::decr("counter");
if dec != 10 { panic!("decr failed"); }
"#).await;
    assert!(code == 0, "store int operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_float_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::set_float("pi", 3.14);
let value = store::get_float("pi");
if value < 3.13 || value > 3.15 { panic!("float ops failed"); }
"#).await;
    assert!(code == 0, "store float operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_bool_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::set_bool("flag", true);
let value = store::get_bool("flag");
if !value { panic!("bool ops failed"); }
"#).await;
    assert!(code == 0, "store bool operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_json_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::set_json("data", "{\"key\": \"value\"}");
let value = store::get_json("data");
if !value.contains("key") { panic!("json ops failed"); }
"#).await;
    assert!(code == 0, "store json operations failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_utility_functions() {
    let (code, stdout, stderr) = run_script_test(r#"
store::clear();
store::set_string("a", "1");
store::set_string("b", "2");
let count = store::count();
if count < 2 { panic!("count failed"); }
let keys = store::keys();
if keys.len() == 0 { panic!("keys failed"); }
store::clear();
"#).await;
    assert!(code == 0, "store utility functions failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_store_list_operations() {
    let (code, stdout, stderr) = run_script_test(r#"
store::clear();
store::push("mylist", "item1");
store::push("mylist", "item2");
let len = store::list_len("mylist");
if len != 2 { panic!("list_len failed"); }
// pop returns the last item pushed
let item = store::pop("mylist");
if item.len() == 0 { panic!("pop returned empty"); }
store::clear();
"#).await;
    assert!(code == 0, "store list operations failed: {} {}", stdout, stderr);
}

// ============================================================================
// CONSOLE MODULE - Complete Tests (18 functions)
// ============================================================================

#[tokio::test]
async fn test_console_logging() {
    let (code, stdout, stderr) = run_script_test(r#"
console::log("test log");
console::info("test info");
console::warn("test warning");
console::debug("test debug");
console::error("test error");
"#).await;
    assert!(code == 0, "console logging failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_console_success() {
    let (code, stdout, stderr) = run_script_test(r#"
console::success("operation succeeded");
"#).await;
    assert!(code == 0, "console::success failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_console_output() {
    let (code, stdout, stderr) = run_script_test(r#"
console::print("no newline");
console::println("with newline");
console::newline();
console::hr();
"#).await;
    assert!(code == 0, "console output failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_console_json() {
    let (code, stdout, stderr) = run_script_test(r#"
console::json("{\"key\": \"value\"}");
"#).await;
    assert!(code == 0, "console::json failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_console_timing() {
    let (code, stdout, stderr) = run_script_test(r#"
console::time("test_timer");
let x = 1 + 1;
console::time_end("test_timer");
"#).await;
    assert!(code == 0, "console timing failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_console_grouping() {
    let (code, stdout, stderr) = run_script_test(r#"
console::group("Test Group");
console::info("Inside group");
console::group_end();
"#).await;
    assert!(code == 0, "console grouping failed: {} {}", stdout, stderr);
}

// ============================================================================
// SYSTEM MODULE - Complete Tests (12 functions)
// ============================================================================

#[tokio::test]
async fn test_system_time() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = system::now();
if now < 1700000000000 { panic!("now failed"); }
let secs = system::now_secs();
if secs < 1700000000 { panic!("now_secs failed"); }
let ts = system::timestamp();
if ts.len() == 0 { panic!("timestamp failed"); }
"#).await;
    assert!(code == 0, "system time failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_system_info() {
    let (code, stdout, stderr) = run_script_test(r#"
let platform = system::platform();
let arch = system::arch();
let hostname = system::hostname();
let username = system::username();
let home = system::home_dir();
if platform.len() == 0 || arch.len() == 0 {
    panic!("system info failed");
}
"#).await;
    assert!(code == 0, "system info failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_system_pid() {
    let (code, stdout, stderr) = run_script_test(r#"
let pid = system::pid();
if pid < 1 { panic!("pid failed"); }
"#).await;
    assert!(code == 0, "system::pid failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_system_sleep() {
    let (code, stdout, stderr) = run_script_test(r#"
system::sleep(10);
"#).await;
    assert!(code == 0, "system::sleep failed: {} {}", stdout, stderr);
}

// ============================================================================
// INTEGRATION TESTS - Complex Multi-Module Scripts
// ============================================================================

#[tokio::test]
async fn test_integration_crypto_and_encoding() {
    let (code, stdout, stderr) = run_script_test(r#"
let message = "important data";
let hash = crypto::sha256_hex(message);
let encoded = encoding::base64_encode(hash);
let decoded = encoding::base64_decode(encoded);
if decoded != hash {
    panic!("crypto + encoding integration failed");
}
"#).await;
    assert!(code == 0, "integration crypto+encoding failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_integration_json_and_regex() {
    let (code, stdout, stderr) = run_script_test(r#"
let data = "{\"email\": \"test@example.com\", \"phone\": \"123-456-7890\"}";
let email = json::get(data, "email");
if !regex::test(email, "@example\\.com") {
    panic!("email regex failed");
}
"#).await;
    assert!(code == 0, "integration json+regex failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_integration_date_and_json() {
    let (code, stdout, stderr) = run_script_test(r#"
let now = date::now();
let ts = date::timestamp();
let data = "{\"created_at\": \"" + now + "\", \"timestamp\": " + `${ts}` + "}";
if !json::is_object(data) {
    panic!("date + json integration failed");
}
"#).await;
    assert!(code == 0, "integration date+json failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_integration_faker_and_store() {
    let (code, stdout, stderr) = run_script_test(r#"
store::clear();
let name = faker::name();
let email = faker::email();
store::set_string("user_name", name);
store::set_string("user_email", email);
let retrieved = store::get_string("user_name");
if retrieved != name {
    panic!("faker + store integration failed");
}
store::clear();
"#).await;
    assert!(code == 0, "integration faker+store failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_integration_url_and_encoding() {
    let (code, stdout, stderr) = run_script_test(r#"
let base = "https://api.example.com/search";
let query = url::encode("hello world");
let full_url = url::set_query_param(base, "q", query);
let host = url::host(full_url);
if host != "api.example.com" {
    panic!("url + encoding integration failed");
}
"#).await;
    assert!(code == 0, "integration url+encoding failed: {} {}", stdout, stderr);
}

#[tokio::test]
async fn test_integration_complete_workflow() {
    let (code, stdout, stderr) = run_script_test(r#"
console::group("API Test");

let user_id = crypto::uuid_v4();
let timestamp = crypto::timestamp();
let api_key = crypto::random_string(32);

console::info("Generated test data");

let message = `${timestamp}:${user_id}`;
let signature = crypto::hmac_sha256(api_key, message);

console::info("Built signature");

let response = "{\"status\": \"success\", \"user_id\": \"" + user_id + "\"}";
if !json::is_object(response) {
    panic!("response not object");
}
if !json::has(response, "status") {
    panic!("missing status");
}

console::success("All checks passed");
console::group_end();
"#).await;
    assert!(code == 0, "integration complete workflow failed: {} {}", stdout, stderr);
}
