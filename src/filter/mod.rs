//! JSON filtering module using jq expressions
//!
//! This module provides JQ-style filtering for JSON responses using the jaq library.

use crate::errors::QuicpulseError;
use jaq_core::load::{Arena, File, Loader};
use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
use jaq_json::Val;
use serde_json::Value as JsonValue;

/// Apply a JQ filter expression to a JSON value
pub fn apply_filter(json: &JsonValue, filter_expr: &str) -> Result<Vec<JsonValue>, QuicpulseError> {
    // Parse the filter
    let filter = parse_filter(filter_expr)?;

    // Parse JSON into jaq Val
    let json_bytes = serde_json::to_vec(json)
        .map_err(|e| QuicpulseError::Argument(format!("JSON serialization error: {}", e)))?;
    let input = jaq_json::read::parse_single(&json_bytes).map_err(|e| {
        QuicpulseError::Argument(format!("Failed to parse JSON for filter: {:?}", e))
    })?;

    // Create execution context
    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));

    // Run the filter and collect results
    let mut output = Vec::new();
    for result in filter.id.run((ctx, input)).map(unwrap_valr) {
        match result {
            Ok(val) => {
                let json_val: JsonValue = serde_json::from_str(&val.to_string())
                    .unwrap_or_else(|_| JsonValue::String(val.to_string()));
                output.push(json_val);
            }
            Err(e) => {
                return Err(QuicpulseError::Argument(format!("Filter error: {:?}", e)));
            }
        }
    }

    Ok(output)
}

/// Parse a JQ filter expression
fn parse_filter(
    expr: &str,
) -> Result<jaq_core::compile::Filter<jaq_core::Native<data::JustLut<Val>>>, QuicpulseError> {
    let arena = Arena::default();
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let loader = Loader::new(defs);
    let path = ();

    let modules = loader
        .load(&arena, File { path, code: expr })
        .map_err(|errs| {
            let msg = errs
                .into_iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            QuicpulseError::Argument(format!("Failed to parse filter: {}", msg))
        })?;

    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    let filter = Compiler::<_, data::JustLut<Val>>::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|errs| {
            let msg = errs
                .into_iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            QuicpulseError::Argument(format!("Failed to compile filter: {}", msg))
        })?;

    Ok(filter)
}

/// Format filtered results for output
pub fn format_filtered_output(results: &[JsonValue], pretty: bool) -> String {
    if results.is_empty() {
        return String::new();
    }

    if results.len() == 1 {
        if pretty {
            serde_json::to_string_pretty(&results[0]).unwrap_or_else(|_| format!("{}", results[0]))
        } else {
            serde_json::to_string(&results[0]).unwrap_or_else(|_| format!("{}", results[0]))
        }
    } else {
        results
            .iter()
            .map(|v| {
                if pretty {
                    serde_json::to_string_pretty(v).unwrap_or_else(|_| format!("{}", v))
                } else {
                    serde_json::to_string(v).unwrap_or_else(|_| format!("{}", v))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_identity_filter() {
        let json = json!({"name": "test", "value": 42});
        let results = apply_filter(&json, ".").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json);
    }

    #[test]
    fn test_field_access() {
        let json = json!({"name": "test", "value": 42});
        let results = apply_filter(&json, ".name").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!("test"));
    }

    #[test]
    fn test_array_access() {
        let json = json!({"items": [1, 2, 3]});
        let results = apply_filter(&json, ".items[0]").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!(1));
    }

    #[test]
    fn test_array_iteration() {
        let json = json!([{"name": "a"}, {"name": "b"}]);
        let results = apply_filter(&json, ".[].name").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!("a"));
        assert_eq!(results[1], json!("b"));
    }

    #[test]
    fn test_pipe() {
        let json = json!({"data": {"users": [{"id": 1}, {"id": 2}]}});
        let results = apply_filter(&json, ".data.users | .[0].id").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!(1));
    }

    #[test]
    fn test_select() {
        let json = json!([1, 2, 3, 4, 5]);
        let results = apply_filter(&json, ".[] | select(. > 3)").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!(4));
        assert_eq!(results[1], json!(5));
    }

    #[test]
    fn test_keys() {
        let json = json!({"a": 1, "b": 2});
        let results = apply_filter(&json, "keys").unwrap();
        assert_eq!(results.len(), 1);
        // Keys may be in any order
        let keys = results[0].as_array().unwrap();
        assert!(keys.contains(&json!("a")));
        assert!(keys.contains(&json!("b")));
    }

    #[test]
    fn test_invalid_filter() {
        let json = json!({"name": "test"});
        let result = apply_filter(&json, ".[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_output_single() {
        let results = vec![json!({"name": "test"})];
        let output = format_filtered_output(&results, false);
        assert_eq!(output, r#"{"name":"test"}"#);
    }

    #[test]
    fn test_format_output_multiple() {
        let results = vec![json!("a"), json!("b")];
        let output = format_filtered_output(&results, false);
        assert_eq!(output, "\"a\"\n\"b\"");
    }
}
