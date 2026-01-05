//! XML module for Rune scripts
//!
//! Provides XML parsing and conversion to JSON.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{json, Value as JsonValue, Map};

/// Create the xml module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("xml")?;

    // Conversion
    module.function("to_json", xml_to_json).build()?;
    module.function("parse", xml_to_json).build()?;

    // Query
    module.function("get_text", get_text_content).build()?;
    module.function("get_attr", get_attribute).build()?;
    module.function("count_elements", count_elements).build()?;
    module.function("has_element", has_element).build()?;

    // Validation
    module.function("is_valid", is_valid_xml).build()?;

    Ok(module)
}

/// Convert XML to JSON representation
fn xml_to_json(xml: &str) -> RuneString {
    let result = parse_xml_to_json(xml);
    RuneString::try_from(result.to_string()).unwrap_or_default()
}

fn parse_xml_to_json(xml: &str) -> JsonValue {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<(String, Map<String, JsonValue>, Vec<JsonValue>)> = Vec::new();
    let mut root: Option<JsonValue> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref()).to_string();
                let mut attrs = Map::new();

                // Parse attributes
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.insert(format!("@{}", key), JsonValue::String(value));
                }

                stack.push((name, attrs, Vec::new()));
            }
            Ok(Event::End(ref e)) => {
                let closing_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if let Some((name, attrs, children)) = stack.pop() {
                    // Check tag name match
                    if name != closing_name {
                        // Tag mismatch - this is malformed XML
                        // Return null to indicate parse error
                        return JsonValue::Null;
                    }
                    let mut obj = Map::new();

                    // Add attributes
                    for (k, v) in attrs {
                        obj.insert(k, v);
                    }

                    // Add children or text content
                    if children.len() == 1 {
                        if let Some(JsonValue::String(s)) = children.first() {
                            if obj.is_empty() {
                                // Simple text element
                                let element = JsonValue::String(s.clone());
                                if let Some((_, _, parent_children)) = stack.last_mut() {
                                    parent_children.push(json!({name: element}));
                                } else {
                                    root = Some(json!({name: element}));
                                }
                                continue;
                            } else {
                                obj.insert("#text".to_string(), JsonValue::String(s.clone()));
                            }
                        } else {
                            obj.insert("#children".to_string(), children.first().unwrap().clone());
                        }
                    } else if !children.is_empty() {
                        obj.insert("#children".to_string(), JsonValue::Array(children));
                    }

                    let element = JsonValue::Object(obj);
                    if let Some((_, _, parent_children)) = stack.last_mut() {
                        parent_children.push(json!({name: element}));
                    } else {
                        root = Some(json!({name: element}));
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                if !text.is_empty() {
                    if let Some((_, _, children)) = stack.last_mut() {
                        children.push(JsonValue::String(text));
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref()).to_string();
                let mut attrs = Map::new();

                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.insert(format!("@{}", key), JsonValue::String(value));
                }

                let element = if attrs.is_empty() {
                    JsonValue::Null
                } else {
                    JsonValue::Object(attrs)
                };

                if let Some((_, _, children)) = stack.last_mut() {
                    children.push(json!({name: element}));
                } else {
                    root = Some(json!({name: element}));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    root.unwrap_or(JsonValue::Null)
}

/// Get text content of first matching element by tag name
fn get_text_content(xml: &str, tag: &str) -> RuneString {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_target = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref());
                if name == tag {
                    in_target = true;
                }
            }
            Ok(Event::Text(ref e)) if in_target => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                return RuneString::try_from(text).unwrap_or_default();
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref());
                if name == tag {
                    in_target = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    RuneString::new()
}

/// Get attribute value from first matching element
fn get_attribute(xml: &str, tag: &str, attr: &str) -> RuneString {
    let mut reader = Reader::from_str(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref());
                if name == tag {
                    for a in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(a.key.as_ref());
                        if key == attr {
                            let value = String::from_utf8_lossy(&a.value).to_string();
                            return RuneString::try_from(value).unwrap_or_default();
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    RuneString::new()
}

/// Count occurrences of an element
fn count_elements(xml: &str, tag: &str) -> i64 {
    let mut reader = Reader::from_str(xml);
    let mut count = 0i64;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let name = String::from_utf8_lossy(name_bytes.as_ref());
                if name == tag {
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    count
}

/// Check if an element exists
fn has_element(xml: &str, tag: &str) -> bool {
    count_elements(xml, tag) > 0
}

/// Check if XML is valid
fn is_valid_xml(xml: &str) -> bool {
    let mut reader = Reader::from_str(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return true,
            Err(_) => return false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_to_json() {
        let xml = r#"<root><item id="1">Hello</item></root>"#;
        let json = xml_to_json(xml);
        assert!(json.contains("root"));
        assert!(json.contains("item"));
    }

    #[test]
    fn test_get_text() {
        let xml = r#"<root><name>John</name><age>30</age></root>"#;
        let name = get_text_content(xml, "name");
        assert_eq!(name.as_str(), "John");
    }

    #[test]
    fn test_get_attribute() {
        let xml = r#"<user id="123" name="John"/>"#;
        let id = get_attribute(xml, "user", "id");
        assert_eq!(id.as_str(), "123");
    }

    #[test]
    fn test_count_elements() {
        let xml = r#"<root><item/><item/><item/></root>"#;
        assert_eq!(count_elements(xml, "item"), 3);
    }

    #[test]
    fn test_is_valid_xml() {
        assert!(is_valid_xml("<root><child/></root>"));
        assert!(!is_valid_xml("<root><child></root>"));
    }
}
