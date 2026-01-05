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
fn format_single_value(value: &JsonValue, options: &JsonFormatterOptions) -> Result<String, String> {
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
    
    String::from_utf8(buf)
        .map_err(|e| format!("UTF-8 error: {}", e))
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
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(|v| sort_json_keys_with_depth(v, depth + 1)).collect())
        }
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
