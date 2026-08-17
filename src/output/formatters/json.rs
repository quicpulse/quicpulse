//! JSON formatting

use serde_json::Value as JsonValue;

/// JSON formatting options
#[derive(Debug, Clone)]
pub struct JsonFormatterOptions {
    /// Indentation (default: 4 spaces)
    pub indent: usize,
    /// Sort keys alphabetically
    pub sort_keys: bool,
}

impl Default for JsonFormatterOptions {
    fn default() -> Self {
        Self {
            indent: 4,
            sort_keys: true,
        }
    }
}

/// Format JSON with pretty printing
/// Supports both single JSON values and NDJSON (newline-delimited JSON)
pub fn format_json(json_str: &str, options: &JsonFormatterOptions) -> Result<String, String> {
    // First, try parsing as a single JSON value
    if let Ok(value) = serde_json::from_str::<JsonValue>(json_str) {
        return format_single_value(&value, options);
    }

    // If that fails, try parsing as NDJSON (newline-delimited JSON)
    let lines: Vec<&str> = json_str.lines().collect();
    let mut results = Vec::new();
    let mut had_valid = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonValue>(trimmed) {
            Ok(value) => {
                had_valid = true;
                results.push(format_single_value(&value, options)?);
            }
            Err(e) => {
                // If we've already parsed some valid JSON, this might be NDJSON
                // with a corrupt line - include it as-is
                if had_valid {
                    results.push(trimmed.to_string());
                } else {
                    // First line isn't valid JSON - fail
                    return Err(format!("Invalid JSON: {}", e));
                }
            }
        }
    }

    if results.is_empty() {
        return Err("Empty JSON input".to_string());
    }

    Ok(results.join("\n"))
}

/// Format a single JSON value
fn format_single_value(
    value: &JsonValue,
    options: &JsonFormatterOptions,
) -> Result<String, String> {
    if options.sort_keys {
        // Sort keys recursively
        let sorted = sort_json_keys(value);
        format_value(&sorted, options.indent)
    } else {
        format_value(value, options.indent)
    }
}

/// Format a JSON value with indentation
fn format_value(value: &JsonValue, indent: usize) -> Result<String, String> {
    let formatter = PrettyFormatter::with_indent(indent);
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);

    serde::Serialize::serialize(value, &mut serializer)
        .map_err(|e| format!("JSON formatting error: {}", e))?;

    String::from_utf8(buf).map_err(|e| format!("UTF-8 error: {}", e))
}

/// Maximum recursion depth for JSON key sorting to prevent stack overflow
const MAX_JSON_DEPTH: usize = 128;

/// Sort JSON object keys recursively with depth limit
fn sort_json_keys(value: &JsonValue) -> JsonValue {
    sort_json_keys_with_depth(value, 0)
}

/// Sort JSON object keys with depth tracking to prevent stack overflow
fn sort_json_keys_with_depth(value: &JsonValue, depth: usize) -> JsonValue {
    // Prevent stack overflow on deeply nested JSON
    if depth >= MAX_JSON_DEPTH {
        return value.clone();
    }

    match value {
        JsonValue::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));

            let sorted_map: serde_json::Map<String, JsonValue> = sorted
                .into_iter()
                .map(|(k, v)| (k.clone(), sort_json_keys_with_depth(v, depth + 1)))
                .collect();

            JsonValue::Object(sorted_map)
        }
        JsonValue::Array(arr) => JsonValue::Array(
            arr.iter()
                .map(|v| sort_json_keys_with_depth(v, depth + 1))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Custom JSON formatter with configurable indentation
struct PrettyFormatter {
    indent: Vec<u8>,
    current_indent: usize,
}

impl PrettyFormatter {
    fn with_indent(spaces: usize) -> Self {
        Self {
            indent: vec![b' '; spaces],
            current_indent: 0,
        }
    }
}

impl serde_json::ser::Formatter for PrettyFormatter {
    fn begin_array<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        self.current_indent += 1;
        writer.write_all(b"[")
    }

    fn end_array<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        self.current_indent -= 1;
        writer.write_all(b"\n")?;
        write_indent(writer, &self.indent, self.current_indent)?;
        writer.write_all(b"]")
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        if first {
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(b",\n")?;
        }
        write_indent(writer, &self.indent, self.current_indent)
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        self.current_indent += 1;
        writer.write_all(b"{")
    }

    fn end_object<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        self.current_indent -= 1;
        writer.write_all(b"\n")?;
        write_indent(writer, &self.indent, self.current_indent)?;
        writer.write_all(b"}")
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        if first {
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(b",\n")?;
        }
        write_indent(writer, &self.indent, self.current_indent)
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        writer.write_all(b": ")
    }
}

fn write_indent<W>(writer: &mut W, indent: &[u8], n: usize) -> std::io::Result<()>
where
    W: ?Sized + std::io::Write,
{
    for _ in 0..n {
        writer.write_all(indent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(s: &str) -> Result<String, String> {
        format_json(s, &JsonFormatterOptions::default())
    }

    #[test]
    fn test_default_options() {
        let o = JsonFormatterOptions::default();
        assert_eq!(o.indent, 4);
        assert!(o.sort_keys);
    }

    #[test]
    fn test_pretty_prints_with_four_space_indent() {
        assert_eq!(fmt(r#"{"a":1}"#).unwrap(), "{\n    \"a\": 1\n}");
    }

    #[test]
    fn test_sorts_keys_recursively() {
        let out = fmt(r#"{"b":1,"a":{"d":2,"c":3}}"#).unwrap();
        assert_eq!(
            out,
            "{\n    \"a\": {\n        \"c\": 3,\n        \"d\": 2\n    },\n    \"b\": 1\n}"
        );
    }

    #[test]
    fn test_sort_keys_disabled_preserves_insertion_order() {
        // Relies on serde_json's preserve_order feature.
        let out = format_json(
            r#"{"b":1,"a":2}"#,
            &JsonFormatterOptions {
                indent: 2,
                sort_keys: false,
            },
        )
        .unwrap();
        assert_eq!(out, "{\n  \"b\": 1,\n  \"a\": 2\n}");
    }

    #[test]
    fn test_custom_indent_width() {
        let out = format_json(
            r#"{"a":{"b":1}}"#,
            &JsonFormatterOptions {
                indent: 2,
                sort_keys: true,
            },
        )
        .unwrap();
        assert_eq!(out, "{\n  \"a\": {\n    \"b\": 1\n  }\n}");
    }

    #[test]
    fn test_arrays_are_expanded_one_element_per_line() {
        assert_eq!(fmt("[1,2,3]").unwrap(), "[\n    1,\n    2,\n    3\n]");
    }

    #[test]
    fn test_nested_array_indentation() {
        assert_eq!(
            fmt(r#"{"a":[1,[2]]}"#).unwrap(),
            "{\n    \"a\": [\n        1,\n        [\n            2\n        ]\n    ]\n}"
        );
    }

    #[test]
    fn test_scalars_pass_through() {
        assert_eq!(fmt("42").unwrap(), "42");
        assert_eq!(fmt("true").unwrap(), "true");
        assert_eq!(fmt("null").unwrap(), "null");
        assert_eq!(fmt(r#""str""#).unwrap(), "\"str\"");
    }

    #[test]
    fn test_ndjson_each_line_formatted() {
        let out = fmt("{\"a\":1}\n{\"b\":2}").unwrap();
        assert_eq!(out, "{\n    \"a\": 1\n}\n{\n    \"b\": 2\n}");
    }

    #[test]
    fn test_ndjson_blank_lines_skipped() {
        let out = fmt("{\"a\":1}\n\n\n{\"b\":2}\n").unwrap();
        assert_eq!(out, "{\n    \"a\": 1\n}\n{\n    \"b\": 2\n}");
    }

    #[test]
    fn test_ndjson_corrupt_trailing_line_passed_through() {
        // Once a valid line is seen, later garbage is preserved verbatim
        // rather than failing the whole stream.
        let out = fmt("{\"a\":1}\nNOTJSON").unwrap();
        assert_eq!(out, "{\n    \"a\": 1\n}\nNOTJSON");
    }

    #[test]
    fn test_invalid_first_line_is_an_error() {
        let err = fmt("NOTJSON").unwrap_err();
        assert!(err.starts_with("Invalid JSON:"), "got: {err}");
    }

    #[test]
    fn test_empty_input_is_an_error() {
        assert_eq!(fmt("").unwrap_err(), "Empty JSON input");
        assert_eq!(fmt("   \n  \t ").unwrap_err(), "Empty JSON input");
    }

    #[test]
    fn test_unicode_and_escapes_round_trip() {
        let out = fmt(r#"{"k":"a\"b\n\tc → é"}"#).unwrap();
        let back: JsonValue = serde_json::from_str(&out).unwrap();
        assert_eq!(back["k"], "a\"b\n\tc → é");
    }

    #[test]
    fn test_output_is_always_reparseable() {
        let src = r#"{"z":[{"y":1},null,true,-1.5e3],"a":{"nested":{"deep":[]}}}"#;
        let out = fmt(src).unwrap();
        let a: JsonValue = serde_json::from_str(src).unwrap();
        let b: JsonValue = serde_json::from_str(&out).unwrap();
        assert_eq!(a, b, "formatting must preserve JSON semantics");
    }

    #[test]
    fn test_duplicate_keys_do_not_panic() {
        let out = fmt(r#"{"a":1,"a":2}"#).unwrap();
        assert!(out.contains("\"a\""), "got: {out}");
    }

    /// Build `{"b":1,"a":{"b":1,"a":{...}}}` nested `depth` levels deep.
    /// Built programmatically because serde_json's own parser caps at 128 levels.
    fn nested_unsorted(depth: usize) -> JsonValue {
        let mut value = JsonValue::Number(0.into());
        for _ in 0..depth {
            let mut map = serde_json::Map::new();
            map.insert("b".to_string(), JsonValue::Number(1.into()));
            map.insert("a".to_string(), value);
            value = JsonValue::Object(map);
        }
        value
    }

    #[test]
    fn test_sort_json_keys_is_depth_limited() {
        let depth = MAX_JSON_DEPTH + 20;
        let sorted = sort_json_keys(&nested_unsorted(depth));

        // Walk down, checking that levels below the cap got sorted and levels
        // at/after it are returned untouched (relies on preserve_order).
        let mut cursor = &sorted;
        for level in 0..MAX_JSON_DEPTH {
            let keys: Vec<_> = cursor.as_object().unwrap().keys().cloned().collect();
            assert_eq!(keys, ["a", "b"], "level {level} should be sorted");
            cursor = &cursor["a"];
        }
        let keys: Vec<_> = cursor.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["b", "a"],
            "level {MAX_JSON_DEPTH} is past the cap and must be left as-is"
        );
    }

    #[test]
    fn test_deep_nesting_does_not_overflow_the_stack() {
        // The depth guard exists to keep this from blowing the stack.
        let value = nested_unsorted(MAX_JSON_DEPTH * 4);
        let sorted = sort_json_keys(&value);
        assert!(sorted.is_object());
    }

    #[test]
    fn test_sort_json_keys_sorts_inside_arrays() {
        let value: JsonValue = serde_json::from_str(r#"[{"b":1,"a":2}]"#).unwrap();
        let sorted = sort_json_keys(&value);
        let keys: Vec<_> = sorted[0].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn test_sort_json_keys_leaves_scalars_untouched() {
        for raw in ["1", "true", "null", r#""s""#] {
            let v: JsonValue = serde_json::from_str(raw).unwrap();
            assert_eq!(sort_json_keys(&v), v);
        }
    }
}
