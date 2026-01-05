//! Table and CSV output formatting
//!
//! This module provides ASCII table and CSV output for JSON arrays.

use comfy_table::{Table, ContentArrangement, Cell, Attribute};
use serde_json::Value as JsonValue;
use std::io::Write;
use crate::errors::QuicpulseError;

/// Format JSON array as ASCII table
pub fn format_as_table(json: &JsonValue) -> Result<String, QuicpulseError> {
    let array = json.as_array()
        .ok_or_else(|| QuicpulseError::Argument("Expected JSON array for table output".to_string()))?;

    if array.is_empty() {
        return Ok("(empty)".to_string());
    }

    // Collect all unique keys from objects
    let columns = collect_columns(array);
    if columns.is_empty() {
        return Err(QuicpulseError::Argument("Array contains no objects".to_string()));
    }

    // Create table
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Add header row
    let header: Vec<Cell> = columns.iter()
        .map(|col| Cell::new(col).add_attribute(Attribute::Bold))
        .collect();
    table.set_header(header);

    // Add data rows
    for item in array {
        if let Some(obj) = item.as_object() {
            let row: Vec<Cell> = columns.iter()
                .map(|col| {
                    let value = obj.get(col)
                        .map(|v| format_cell_value(v))
                        .unwrap_or_else(|| "".to_string());
                    Cell::new(value)
                })
                .collect();
            table.add_row(row);
        }
    }

    Ok(table.to_string())
}

/// Format JSON array as CSV
pub fn format_as_csv(json: &JsonValue) -> Result<String, QuicpulseError> {
    let array = json.as_array()
        .ok_or_else(|| QuicpulseError::Argument("Expected JSON array for CSV output".to_string()))?;

    if array.is_empty() {
        return Ok(String::new());
    }

    // Collect all unique keys from objects
    let columns = collect_columns(array);
    if columns.is_empty() {
        return Err(QuicpulseError::Argument("Array contains no objects".to_string()));
    }

    let mut output = Vec::new();
    {
        let mut writer = csv::Writer::from_writer(&mut output);

        // Write header
        writer.write_record(&columns)
            .map_err(|e| QuicpulseError::Argument(format!("CSV write error: {}", e)))?;

        // Write data rows
        for item in array {
            if let Some(obj) = item.as_object() {
                let row: Vec<String> = columns.iter()
                    .map(|col| {
                        obj.get(col)
                            .map(|v| format_csv_value(v))
                            .unwrap_or_default()
                    })
                    .collect();
                writer.write_record(&row)
                    .map_err(|e| QuicpulseError::Argument(format!("CSV write error: {}", e)))?;
            }
        }

        writer.flush()
            .map_err(|e| QuicpulseError::Argument(format!("CSV flush error: {}", e)))?;
    }

    String::from_utf8(output)
        .map_err(|e| QuicpulseError::Argument(format!("UTF-8 error: {}", e)))
}

/// Write CSV directly to a writer
pub fn write_csv<W: Write>(json: &JsonValue, writer: W) -> Result<(), QuicpulseError> {
    let array = json.as_array()
        .ok_or_else(|| QuicpulseError::Argument("Expected JSON array for CSV output".to_string()))?;

    if array.is_empty() {
        return Ok(());
    }

    let columns = collect_columns(array);
    if columns.is_empty() {
        return Err(QuicpulseError::Argument("Array contains no objects".to_string()));
    }

    let mut csv_writer = csv::Writer::from_writer(writer);

    // Write header
    csv_writer.write_record(&columns)
        .map_err(|e| QuicpulseError::Argument(format!("CSV write error: {}", e)))?;

    // Write data rows
    for item in array {
        if let Some(obj) = item.as_object() {
            let row: Vec<String> = columns.iter()
                .map(|col| {
                    obj.get(col)
                        .map(|v| format_csv_value(v))
                        .unwrap_or_default()
                })
                .collect();
            csv_writer.write_record(&row)
                .map_err(|e| QuicpulseError::Argument(format!("CSV write error: {}", e)))?;
        }
    }

    csv_writer.flush()
        .map_err(|e| QuicpulseError::Argument(format!("CSV flush error: {}", e)))?;

    Ok(())
}

/// Collect unique column names from array of objects
fn collect_columns(array: &[JsonValue]) -> Vec<String> {
    let mut columns = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in array {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }

    columns
}

/// Format a JSON value for table cell display
fn format_cell_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => {
            if arr.len() <= 3 {
                format!("[{}]", arr.iter()
                    .map(|v| format_cell_value(v))
                    .collect::<Vec<_>>()
                    .join(", "))
            } else {
                format!("[{} items]", arr.len())
            }
        }
        JsonValue::Object(obj) => {
            if obj.len() <= 2 {
                format!("{{{}}}", obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, format_cell_value(v)))
                    .collect::<Vec<_>>()
                    .join(", "))
            } else {
                format!("{{...{} keys}}", obj.len())
            }
        }
    }
}

/// Format a JSON value for CSV output
fn format_csv_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
        JsonValue::Object(obj) => serde_json::to_string(obj).unwrap_or_default(),
    }
}

/// Check if JSON can be formatted as a table
pub fn can_format_as_table(json: &JsonValue) -> bool {
    if let Some(array) = json.as_array() {
        // Must have at least one object
        array.iter().any(|item| item.is_object())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_as_table() {
        let json = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let table = format_as_table(&json).unwrap();
        assert!(table.contains("Alice"));
        assert!(table.contains("Bob"));
        assert!(table.contains("name"));
        assert!(table.contains("age"));
    }

    #[test]
    fn test_format_as_csv() {
        let json = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let csv = format_as_csv(&json).unwrap();
        assert!(csv.contains("name"));
        assert!(csv.contains("age"));
        assert!(csv.contains("Alice"));
        assert!(csv.contains("30"));
    }

    #[test]
    fn test_empty_array() {
        let json = json!([]);
        let result = format_as_table(&json).unwrap();
        assert_eq!(result, "(empty)");
    }

    #[test]
    fn test_non_array() {
        let json = json!({"name": "test"});
        let result = format_as_table(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_columns() {
        let json = json!([
            {"name": "Alice", "email": "alice@example.com"},
            {"name": "Bob", "phone": "555-1234"}
        ]);

        let table = format_as_table(&json).unwrap();
        assert!(table.contains("name"));
        assert!(table.contains("email"));
        assert!(table.contains("phone"));
    }

    #[test]
    fn test_nested_values() {
        let json = json!([
            {"name": "Alice", "tags": ["a", "b"]},
            {"name": "Bob", "meta": {"role": "admin"}}
        ]);

        let table = format_as_table(&json).unwrap();
        assert!(table.contains("Alice"));
        assert!(table.contains("[a, b]"));
    }

    #[test]
    fn test_can_format_as_table() {
        assert!(can_format_as_table(&json!([{"a": 1}])));
        assert!(!can_format_as_table(&json!({"a": 1})));
        assert!(!can_format_as_table(&json!([1, 2, 3])));
    }

    #[test]
    fn test_format_cell_value() {
        assert_eq!(format_cell_value(&json!(null)), "null");
        assert_eq!(format_cell_value(&json!(true)), "true");
        assert_eq!(format_cell_value(&json!(42)), "42");
        assert_eq!(format_cell_value(&json!("hello")), "hello");
    }
}
