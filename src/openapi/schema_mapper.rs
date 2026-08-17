//! Schema to Magic Value Mapper
//!
//! Maps OpenAPI schema types and formats to QuicPulse magic.rs template values.

use super::parser::Schema;
use serde_json::Value;
use std::collections::HashMap;

/// Maps OpenAPI schemas to magic.rs template values
pub struct SchemaMapper;

impl SchemaMapper {
    /// Generate a magic value template based on schema type and format
    pub fn schema_to_magic(schema: &Schema) -> String {
        // Handle enum values first
        if !schema.enum_values.is_empty() {
            return Self::enum_to_magic(&schema.enum_values);
        }

        // Handle example if provided
        if let Some(example) = &schema.example {
            return Self::value_to_string(example);
        }

        // Handle default if provided
        if let Some(default) = &schema.default {
            return Self::value_to_string(default);
        }

        // Map type + format to magic values
        match (schema.schema_type.as_deref(), schema.format.as_deref()) {
            // String formats
            (Some("string"), Some("uuid")) => "{uuid}".to_string(),
            (Some("string"), Some("email")) => "{email}".to_string(),
            (Some("string"), Some("date-time")) => "{now}".to_string(),
            (Some("string"), Some("date")) => "{date}".to_string(),
            (Some("string"), Some("time")) => "{time}".to_string(),
            (Some("string"), Some("uri")) => "https://example.com/{random_string:8}".to_string(),
            (Some("string"), Some("hostname")) => "example-{random_string:8}.com".to_string(),
            (Some("string"), Some("ipv4")) => "192.168.1.{random_int:1:254}".to_string(),
            (Some("string"), Some("ipv6")) => "::1".to_string(),
            (Some("string"), Some("byte")) => "{random_bytes:16}".to_string(),
            (Some("string"), Some("binary")) => "{random_bytes:32}".to_string(),
            (Some("string"), Some("password")) => "{random_string:16}".to_string(),
            (Some("string"), Some("phone")) => {
                "+1-555-{random_int:100:999}-{random_int:1000:9999}".to_string()
            }

            // String with length constraints
            (Some("string"), _) => Self::string_with_constraints(schema),

            // Integer formats
            (Some("integer"), Some("int32")) => {
                Self::int_with_constraints(schema, i32::MIN as i64, i32::MAX as i64)
            }
            (Some("integer"), Some("int64")) => {
                Self::int_with_constraints(schema, i64::MIN, i64::MAX)
            }
            // No format means no narrower type bound than i64. The 0/1000
            // defaults are applied inside int_with_constraints when the schema
            // omits minimum/maximum; passing them as clamps here would discard
            // a legitimate negative minimum.
            (Some("integer"), _) => Self::int_with_constraints(schema, i64::MIN, i64::MAX),

            // Number formats
            (Some("number"), Some("float")) => Self::float_with_constraints(schema),
            (Some("number"), Some("double")) => Self::float_with_constraints(schema),
            (Some("number"), _) => Self::float_with_constraints(schema),

            // Boolean
            (Some("boolean"), _) => "{random_bool}".to_string(),

            // Array
            (Some("array"), _) => Self::array_to_magic(schema),

            // Object
            (Some("object"), _) => Self::object_to_magic(schema),

            // Unknown type - use random string
            _ => "{random_string:10}".to_string(),
        }
    }

    /// Convert enum values to a pick template
    fn enum_to_magic(values: &[Value]) -> String {
        let options: Vec<String> = values.iter().map(Self::value_to_string).collect();
        format!("{{pick:{}}}", options.join(","))
    }

    /// Generate string with length constraints
    fn string_with_constraints(schema: &Schema) -> String {
        let len = match (schema.min_length, schema.max_length) {
            (Some(min), Some(max)) => (min + max) / 2,
            (Some(min), None) => min.max(10),
            (None, Some(max)) => max.min(20),
            (None, None) => 10,
        };
        format!("{{random_string:{}}}", len)
    }

    /// Generate integer with constraints
    fn int_with_constraints(schema: &Schema, type_min: i64, type_max: i64) -> String {
        /// Bounds used for whichever side the schema leaves unspecified.
        const DEFAULT_MIN: i64 = 0;
        const DEFAULT_MAX: i64 = 1000;

        let (mut min, mut max) = match (schema.minimum, schema.maximum) {
            (Some(lo), Some(hi)) => (lo as i64, hi as i64),
            // Stretch the defaulted side only as far as needed to stay on the
            // correct side of the bound the schema did give.
            (Some(lo), None) => {
                let lo = lo as i64;
                (lo, lo.max(DEFAULT_MAX))
            }
            (None, Some(hi)) => {
                let hi = hi as i64;
                (hi.min(DEFAULT_MIN), hi)
            }
            (None, None) => (DEFAULT_MIN, DEFAULT_MAX),
        };

        // Repair a schema that states its bounds backwards.
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }

        // Clamp into the range the format can actually represent.
        min = min.clamp(type_min, type_max);
        max = max.clamp(type_min, type_max);

        format!("{{random_int:{}:{}}}", min, max)
    }

    /// Generate float with constraints
    fn float_with_constraints(schema: &Schema) -> String {
        let min = schema.minimum.unwrap_or(0.0);
        let max = schema.maximum.unwrap_or(100.0);
        format!("{{random_float:{}:{}}}", min, max)
    }

    /// Generate array magic value
    fn array_to_magic(schema: &Schema) -> String {
        if let Some(items) = &schema.items {
            let item_value = Self::schema_to_magic(items);
            // Generate a single-element array template
            format!("[{}]", item_value)
        } else {
            "[]".to_string()
        }
    }

    /// Generate object magic value
    fn object_to_magic(schema: &Schema) -> String {
        if schema.properties.is_empty() {
            return "{}".to_string();
        }

        let mut pairs = Vec::new();
        for (name, prop_schema) in &schema.properties {
            let value = Self::schema_to_magic(prop_schema);
            // Determine if value needs quoting
            let quoted_value = if value.starts_with('{') && !value.starts_with("{\"") {
                format!("\"{}\"", value)
            } else if value.starts_with('[') || value.starts_with('{') {
                value
            } else if value.parse::<f64>().is_ok() || value == "true" || value == "false" {
                value
            } else {
                format!("\"{}\"", value)
            };
            pairs.push(format!("\"{}\": {}", name, quoted_value));
        }
        format!("{{{}}}", pairs.join(", "))
    }

    /// Convert a JSON value to string
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string()),
            Value::Object(obj) => serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    /// Generate a complete request body from a schema
    pub fn generate_request_body(schema: &Schema, schemas: &HashMap<String, Schema>) -> Value {
        Self::generate_value(schema, schemas, 0)
    }

    /// Generate a JSON value from a schema, with recursion limit
    fn generate_value(schema: &Schema, schemas: &HashMap<String, Schema>, depth: usize) -> Value {
        // Prevent infinite recursion
        if depth > 10 {
            return Value::Null;
        }

        // Handle $ref
        if let Some(ref_path) = &schema.ref_path {
            let ref_name = ref_path.rsplit('/').next().unwrap_or("");
            if let Some(ref_schema) = schemas.get(ref_name) {
                return Self::generate_value(ref_schema, schemas, depth + 1);
            }
        }

        // Handle example
        if let Some(example) = &schema.example {
            return example.clone();
        }

        // Handle enum
        if !schema.enum_values.is_empty() {
            if let Some(first) = schema.enum_values.first() {
                return first.clone();
            }
        }

        match schema.schema_type.as_deref() {
            Some("object") => {
                let mut obj = serde_json::Map::new();
                for (name, prop_schema) in &schema.properties {
                    let magic_or_value = Self::schema_to_magic(prop_schema);
                    // Check if it's a magic value or actual value
                    if magic_or_value.contains('{') && magic_or_value.contains('}') {
                        // It's a magic template, use as string
                        obj.insert(name.clone(), Value::String(magic_or_value));
                    } else {
                        // Try to parse as JSON, fallback to string
                        let value = serde_json::from_str(&magic_or_value)
                            .unwrap_or_else(|_| Value::String(magic_or_value));
                        obj.insert(name.clone(), value);
                    }
                }
                Value::Object(obj)
            }
            Some("array") => {
                if let Some(items) = &schema.items {
                    let item = Self::generate_value(items, schemas, depth + 1);
                    Value::Array(vec![item])
                } else {
                    Value::Array(vec![])
                }
            }
            Some("string") => Value::String(Self::schema_to_magic(schema)),
            Some("integer") | Some("number") => {
                // Return the magic template as a string - will be expanded later
                Value::String(Self::schema_to_magic(schema))
            }
            Some("boolean") => Value::String("{random_bool}".to_string()),
            _ => Value::Null,
        }
    }

    /// Map OpenAPI type to fuzz category for automated security testing
    pub fn type_to_fuzz_category(schema: &Schema) -> Vec<String> {
        let mut categories = Vec::new();

        match (schema.schema_type.as_deref(), schema.format.as_deref()) {
            // String types vulnerable to injection
            (Some("string"), None) => {
                categories.push("sql".to_string());
                categories.push("xss".to_string());
                categories.push("cmd".to_string());
            }
            (Some("string"), Some("uri" | "url")) => {
                categories.push("ssrf".to_string());
                categories.push("path".to_string());
            }
            (Some("string"), Some("email")) => {
                categories.push("format".to_string());
            }

            // Integer types - boundary testing
            (Some("integer"), _) => {
                categories.push("int".to_string());
                categories.push("boundary".to_string());
            }

            // All types get type confusion tests
            _ => {
                categories.push("type".to_string());
            }
        }

        categories
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("uuid".to_string()),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{uuid}");
    }

    #[test]
    fn test_email_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("email".to_string()),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{email}");
    }

    #[test]
    fn test_integer_with_bounds() {
        let schema = Schema {
            schema_type: Some("integer".to_string()),
            minimum: Some(1.0),
            maximum: Some(100.0),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "{random_int:1:100}");
    }

    #[test]
    fn test_enum_mapping() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            enum_values: vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string()),
            ],
            ..Default::default()
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&schema),
            "{pick:active,inactive}"
        );
    }

    #[test]
    fn test_example_takes_precedence() {
        let schema = Schema {
            schema_type: Some("string".to_string()),
            example: Some(Value::String("my-example".to_string())),
            ..Default::default()
        };
        assert_eq!(SchemaMapper::schema_to_magic(&schema), "my-example");
    }

    fn typed(t: &str) -> Schema {
        Schema {
            schema_type: Some(t.to_string()),
            ..Default::default()
        }
    }

    fn formatted(t: &str, f: &str) -> Schema {
        Schema {
            schema_type: Some(t.to_string()),
            format: Some(f.to_string()),
            ..Default::default()
        }
    }

    // ---- string formats ----

    #[test]
    fn test_all_string_formats_map_to_magic_values() {
        let cases = [
            ("uuid", "{uuid}"),
            ("email", "{email}"),
            ("date-time", "{now}"),
            ("date", "{date}"),
            ("time", "{time}"),
            ("uri", "https://example.com/{random_string:8}"),
            ("hostname", "example-{random_string:8}.com"),
            ("ipv4", "192.168.1.{random_int:1:254}"),
            ("ipv6", "::1"),
            ("byte", "{random_bytes:16}"),
            ("binary", "{random_bytes:32}"),
            ("password", "{random_string:16}"),
            (
                "phone",
                "+1-555-{random_int:100:999}-{random_int:1000:9999}",
            ),
        ];
        for (format, expected) in cases {
            assert_eq!(
                SchemaMapper::schema_to_magic(&formatted("string", format)),
                expected,
                "format {format}"
            );
        }
    }

    #[test]
    fn test_plain_string_defaults_to_ten_chars() {
        assert_eq!(
            SchemaMapper::schema_to_magic(&typed("string")),
            "{random_string:10}"
        );
    }

    #[test]
    fn test_string_length_constraints() {
        // Both bounds -> midpoint.
        let both = Schema {
            min_length: Some(4),
            max_length: Some(10),
            ..typed("string")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&both), "{random_string:7}");

        // Min only -> at least 10.
        let min_small = Schema {
            min_length: Some(3),
            ..typed("string")
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&min_small),
            "{random_string:10}"
        );

        let min_large = Schema {
            min_length: Some(25),
            ..typed("string")
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&min_large),
            "{random_string:25}"
        );

        // Max only -> capped at 20.
        let max_large = Schema {
            max_length: Some(50),
            ..typed("string")
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&max_large),
            "{random_string:20}"
        );

        let max_small = Schema {
            max_length: Some(5),
            ..typed("string")
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&max_small),
            "{random_string:5}"
        );
    }

    #[test]
    fn test_unknown_string_format_uses_length_constraints() {
        // An unrecognized format falls through to the constraint path.
        assert_eq!(
            SchemaMapper::schema_to_magic(&formatted("string", "some-custom-format")),
            "{random_string:10}"
        );
    }

    // ---- numeric ----

    #[test]
    fn test_integer_default_range() {
        assert_eq!(
            SchemaMapper::schema_to_magic(&typed("integer")),
            "{random_int:0:1000}"
        );
    }

    #[test]
    fn test_int32_and_int64_clamp_to_type_bounds() {
        // Without explicit bounds the defaults (0..1000) sit inside both types.
        assert_eq!(
            SchemaMapper::schema_to_magic(&formatted("integer", "int32")),
            "{random_int:0:1000}"
        );
        assert_eq!(
            SchemaMapper::schema_to_magic(&formatted("integer", "int64")),
            "{random_int:0:1000}"
        );

        // An out-of-range maximum is clamped down to i32::MAX.
        let huge = Schema {
            minimum: Some(0.0),
            maximum: Some(1e18),
            ..formatted("integer", "int32")
        };
        assert_eq!(
            SchemaMapper::schema_to_magic(&huge),
            format!("{{random_int:0:{}}}", i32::MAX)
        );
    }

    #[test]
    fn test_negative_integer_bounds_are_preserved() {
        // Regression: a format-less integer used to clamp `minimum` to 0,
        // producing the inverted range "{random_int:0:-10}".
        let s = Schema {
            minimum: Some(-50.0),
            maximum: Some(-10.0),
            ..typed("integer")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "{random_int:-50:-10}");
    }

    #[test]
    fn test_generated_int_ranges_are_never_inverted() {
        let bounds = [
            (Some(-100.0), Some(-1.0)),
            (Some(-5.0), Some(5.0)),
            (Some(0.0), Some(0.0)),
            (Some(10.0), Some(20.0)),
            (None, None),
            (Some(-1.0), None),
            (None, Some(-1.0)),
        ];
        for format in [None, Some("int32"), Some("int64")] {
            for (minimum, maximum) in bounds {
                let schema = Schema {
                    schema_type: Some("integer".to_string()),
                    format: format.map(str::to_string),
                    minimum,
                    maximum,
                    ..Default::default()
                };
                let out = SchemaMapper::schema_to_magic(&schema);
                let body = out.trim_start_matches("{random_int:").trim_end_matches('}');
                // Split on the ':' that separates the two (possibly negative) bounds.
                let idx = body[1..].find(':').unwrap() + 1;
                let lo: i64 = body[..idx].parse().unwrap();
                let hi: i64 = body[idx + 1..].parse().unwrap();
                assert!(
                    lo <= hi,
                    "inverted range {out} for format={format:?} bounds=({minimum:?},{maximum:?})"
                );
            }
        }
    }

    #[test]
    fn test_number_formats_all_map_to_random_float() {
        for f in ["float", "double"] {
            assert_eq!(
                SchemaMapper::schema_to_magic(&formatted("number", f)),
                "{random_float:0:100}"
            );
        }
        assert_eq!(
            SchemaMapper::schema_to_magic(&typed("number")),
            "{random_float:0:100}"
        );
    }

    #[test]
    fn test_number_respects_explicit_bounds() {
        let s = Schema {
            minimum: Some(1.5),
            maximum: Some(2.5),
            ..typed("number")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "{random_float:1.5:2.5}");
    }

    #[test]
    fn test_boolean_maps_to_random_bool() {
        assert_eq!(
            SchemaMapper::schema_to_magic(&typed("boolean")),
            "{random_bool}"
        );
    }

    #[test]
    fn test_unknown_type_falls_back_to_random_string() {
        assert_eq!(
            SchemaMapper::schema_to_magic(&Schema::default()),
            "{random_string:10}"
        );
        assert_eq!(
            SchemaMapper::schema_to_magic(&typed("something-else")),
            "{random_string:10}"
        );
    }

    // ---- composites ----

    #[test]
    fn test_array_wraps_its_item_template() {
        let s = Schema {
            items: Some(Box::new(formatted("string", "uuid"))),
            ..typed("array")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "[{uuid}]");
    }

    #[test]
    fn test_array_without_items_is_empty() {
        assert_eq!(SchemaMapper::schema_to_magic(&typed("array")), "[]");
    }

    #[test]
    fn test_object_without_properties_is_empty() {
        assert_eq!(SchemaMapper::schema_to_magic(&typed("object")), "{}");
    }

    #[test]
    fn test_object_quotes_magic_templates_but_not_numbers_or_bools() {
        let mut props = HashMap::new();
        props.insert("id".to_string(), formatted("string", "uuid"));
        props.insert("flag".to_string(), typed("boolean"));
        let s = Schema {
            properties: props,
            ..typed("object")
        };

        let out = SchemaMapper::schema_to_magic(&s);
        // Magic templates are wrapped in quotes so the result stays valid JSON-ish.
        assert!(out.contains(r#""id": "{uuid}""#), "got: {out}");
        assert!(out.contains(r#""flag": "{random_bool}""#), "got: {out}");
    }

    #[test]
    fn test_object_leaves_literal_scalars_unquoted() {
        let mut props = HashMap::new();
        props.insert(
            "count".to_string(),
            Schema {
                example: Some(Value::Number(7.into())),
                ..typed("integer")
            },
        );
        props.insert(
            "on".to_string(),
            Schema {
                example: Some(Value::Bool(true)),
                ..typed("boolean")
            },
        );
        let s = Schema {
            properties: props,
            ..typed("object")
        };

        let out = SchemaMapper::schema_to_magic(&s);
        assert!(out.contains(r#""count": 7"#), "got: {out}");
        assert!(out.contains(r#""on": true"#), "got: {out}");
    }

    #[test]
    fn test_object_keeps_nested_arrays_unquoted() {
        let mut props = HashMap::new();
        props.insert(
            "tags".to_string(),
            Schema {
                items: Some(Box::new(typed("string"))),
                ..typed("array")
            },
        );
        let s = Schema {
            properties: props,
            ..typed("object")
        };
        let out = SchemaMapper::schema_to_magic(&s);
        assert!(
            out.contains(r#""tags": [{random_string:10}]"#),
            "got: {out}"
        );
    }

    // ---- precedence & value conversion ----

    #[test]
    fn test_enum_beats_example_and_default() {
        let s = Schema {
            enum_values: vec![Value::String("a".into())],
            example: Some(Value::String("ex".into())),
            default: Some(Value::String("def".into())),
            ..typed("string")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "{pick:a}");
    }

    #[test]
    fn test_default_used_when_no_example() {
        let s = Schema {
            default: Some(Value::String("fallback".into())),
            ..typed("string")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "fallback");
    }

    #[test]
    fn test_enum_of_mixed_value_kinds() {
        let s = Schema {
            enum_values: vec![
                Value::Number(1.into()),
                Value::Bool(false),
                Value::Null,
                Value::String("x".into()),
            ],
            ..typed("string")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&s), "{pick:1,false,null,x}");
    }

    #[test]
    fn test_example_of_array_and_object_is_serialized() {
        let arr = Schema {
            example: Some(serde_json::json!([1, 2])),
            ..typed("array")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&arr), "[1,2]");

        let obj = Schema {
            example: Some(serde_json::json!({"k": 1})),
            ..typed("object")
        };
        assert_eq!(SchemaMapper::schema_to_magic(&obj), r#"{"k":1}"#);
    }

    // ---- generate_request_body ----

    #[test]
    fn test_generate_request_body_builds_an_object() {
        let mut props = HashMap::new();
        props.insert("id".to_string(), formatted("string", "uuid"));
        let schema = Schema {
            properties: props,
            ..typed("object")
        };

        let body = SchemaMapper::generate_request_body(&schema, &HashMap::new());
        assert_eq!(body["id"], Value::String("{uuid}".to_string()));
    }

    #[test]
    fn test_generate_request_body_resolves_refs() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), typed("string"));
        let user = Schema {
            properties: props,
            ..typed("object")
        };

        let mut schemas = HashMap::new();
        schemas.insert("User".to_string(), user);

        let referring = Schema {
            ref_path: Some("#/components/schemas/User".to_string()),
            ..Default::default()
        };

        let body = SchemaMapper::generate_request_body(&referring, &schemas);
        assert_eq!(
            body["name"],
            Value::String("{random_string:10}".to_string())
        );
    }

    #[test]
    fn test_generate_request_body_unresolvable_ref_yields_null() {
        let referring = Schema {
            ref_path: Some("#/components/schemas/Missing".to_string()),
            ..Default::default()
        };
        assert_eq!(
            SchemaMapper::generate_request_body(&referring, &HashMap::new()),
            Value::Null
        );
    }

    #[test]
    fn test_generate_request_body_stops_on_self_referencing_ref() {
        // A schema that points at itself must hit the depth guard, not recurse forever.
        let mut schemas = HashMap::new();
        schemas.insert(
            "Node".to_string(),
            Schema {
                ref_path: Some("#/components/schemas/Node".to_string()),
                ..Default::default()
            },
        );
        let root = Schema {
            ref_path: Some("#/components/schemas/Node".to_string()),
            ..Default::default()
        };
        assert_eq!(
            SchemaMapper::generate_request_body(&root, &schemas),
            Value::Null
        );
    }

    #[test]
    fn test_generate_request_body_example_and_enum_precedence() {
        let with_example = Schema {
            example: Some(serde_json::json!({"a": 1})),
            ..typed("object")
        };
        assert_eq!(
            SchemaMapper::generate_request_body(&with_example, &HashMap::new()),
            serde_json::json!({"a": 1})
        );

        let with_enum = Schema {
            enum_values: vec![
                Value::String("first".into()),
                Value::String("second".into()),
            ],
            ..typed("string")
        };
        assert_eq!(
            SchemaMapper::generate_request_body(&with_enum, &HashMap::new()),
            Value::String("first".to_string())
        );
    }

    #[test]
    fn test_generate_request_body_arrays() {
        let with_items = Schema {
            items: Some(Box::new(typed("string"))),
            ..typed("array")
        };
        let body = SchemaMapper::generate_request_body(&with_items, &HashMap::new());
        assert_eq!(body.as_array().unwrap().len(), 1);

        let empty = typed("array");
        assert_eq!(
            SchemaMapper::generate_request_body(&empty, &HashMap::new()),
            Value::Array(vec![])
        );
    }

    #[test]
    fn test_generate_request_body_scalar_types() {
        let schemas = HashMap::new();
        assert_eq!(
            SchemaMapper::generate_request_body(&typed("string"), &schemas),
            Value::String("{random_string:10}".to_string())
        );
        assert_eq!(
            SchemaMapper::generate_request_body(&typed("integer"), &schemas),
            Value::String("{random_int:0:1000}".to_string())
        );
        assert_eq!(
            SchemaMapper::generate_request_body(&typed("number"), &schemas),
            Value::String("{random_float:0:100}".to_string())
        );
        assert_eq!(
            SchemaMapper::generate_request_body(&typed("boolean"), &schemas),
            Value::String("{random_bool}".to_string())
        );
        assert_eq!(
            SchemaMapper::generate_request_body(&Schema::default(), &schemas),
            Value::Null
        );
    }

    #[test]
    fn test_generate_request_body_parses_literal_property_values() {
        // A property whose template contains no braces is parsed as JSON.
        let mut props = HashMap::new();
        props.insert(
            "n".to_string(),
            Schema {
                example: Some(Value::Number(5.into())),
                ..typed("integer")
            },
        );
        let schema = Schema {
            properties: props,
            ..typed("object")
        };
        let body = SchemaMapper::generate_request_body(&schema, &HashMap::new());
        assert_eq!(body["n"], Value::Number(5.into()));
    }

    // ---- fuzz categories ----

    #[test]
    fn test_fuzz_categories_for_plain_string() {
        let cats = SchemaMapper::type_to_fuzz_category(&typed("string"));
        assert_eq!(cats, ["sql", "xss", "cmd"]);
    }

    #[test]
    fn test_fuzz_categories_for_uri_and_url_formats() {
        for f in ["uri", "url"] {
            let cats = SchemaMapper::type_to_fuzz_category(&formatted("string", f));
            assert_eq!(cats, ["ssrf", "path"], "format {f}");
        }
    }

    #[test]
    fn test_fuzz_categories_for_email() {
        let cats = SchemaMapper::type_to_fuzz_category(&formatted("string", "email"));
        assert_eq!(cats, ["format"]);
    }

    #[test]
    fn test_fuzz_categories_for_integers() {
        assert_eq!(
            SchemaMapper::type_to_fuzz_category(&typed("integer")),
            ["int", "boundary"]
        );
        assert_eq!(
            SchemaMapper::type_to_fuzz_category(&formatted("integer", "int64")),
            ["int", "boundary"]
        );
    }

    #[test]
    fn test_fuzz_categories_fall_back_to_type_confusion() {
        for schema in [typed("boolean"), typed("object"), Schema::default()] {
            assert_eq!(SchemaMapper::type_to_fuzz_category(&schema), ["type"]);
        }
        // A string with some other format also lands in the catch-all.
        assert_eq!(
            SchemaMapper::type_to_fuzz_category(&formatted("string", "uuid")),
            ["type"]
        );
    }
}
