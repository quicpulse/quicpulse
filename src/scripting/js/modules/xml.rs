//! XML module for JavaScript
//!
//! Provides XML parsing and conversion utilities.

use rquickjs::{Ctx, Object, Function};
use quick_xml::Reader;
use quick_xml::events::Event;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let xml = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create xml object: {}", e)))?;

    xml.set("parse", Function::new(ctx.clone(), xml_parse)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    xml.set("is_valid", Function::new(ctx.clone(), xml_is_valid)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    xml.set("to_json", Function::new(ctx.clone(), xml_to_json)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    xml.set("get_text", Function::new(ctx.clone(), xml_get_text)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("xml", xml)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set xml global: {}", e)))?;

    Ok(())
}

/// Simple XML to JSON conversion
fn xml_to_json_value(xml_str: &str) -> Result<serde_json::Value, String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<(String, serde_json::Map<String, serde_json::Value>)> = vec![];
    let mut root: Option<serde_json::Value> = None;
    let mut current_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = serde_json::Map::new();

                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.insert(format!("@{}", key), serde_json::Value::String(value));
                }

                stack.push((name, attrs));
                current_text.clear();
            }
            Ok(Event::End(_)) => {
                if let Some((name, mut obj)) = stack.pop() {
                    if !current_text.trim().is_empty() {
                        if obj.is_empty() {
                            // Just text content
                            let value = serde_json::Value::String(current_text.trim().to_string());
                            if let Some((_, parent)) = stack.last_mut() {
                                add_to_object(parent, &name, value);
                            } else {
                                let mut wrapper = serde_json::Map::new();
                                wrapper.insert(name, value);
                                root = Some(serde_json::Value::Object(wrapper));
                            }
                        } else {
                            obj.insert("#text".to_string(), serde_json::Value::String(current_text.trim().to_string()));
                            let value = serde_json::Value::Object(obj);
                            if let Some((_, parent)) = stack.last_mut() {
                                add_to_object(parent, &name, value);
                            } else {
                                let mut wrapper = serde_json::Map::new();
                                wrapper.insert(name, value);
                                root = Some(serde_json::Value::Object(wrapper));
                            }
                        }
                    } else if obj.is_empty() {
                        let value = serde_json::Value::Null;
                        if let Some((_, parent)) = stack.last_mut() {
                            add_to_object(parent, &name, value);
                        } else {
                            let mut wrapper = serde_json::Map::new();
                            wrapper.insert(name, value);
                            root = Some(serde_json::Value::Object(wrapper));
                        }
                    } else {
                        let value = serde_json::Value::Object(obj);
                        if let Some((_, parent)) = stack.last_mut() {
                            add_to_object(parent, &name, value);
                        } else {
                            let mut wrapper = serde_json::Map::new();
                            wrapper.insert(name, value);
                            root = Some(serde_json::Value::Object(wrapper));
                        }
                    }
                }
                current_text.clear();
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = serde_json::Map::new();

                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.insert(format!("@{}", key), serde_json::Value::String(value));
                }

                let value = if attrs.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::Object(attrs)
                };

                if let Some((_, parent)) = stack.last_mut() {
                    add_to_object(parent, &name, value);
                } else {
                    let mut wrapper = serde_json::Map::new();
                    wrapper.insert(name, value);
                    root = Some(serde_json::Value::Object(wrapper));
                }
            }
            Ok(Event::Text(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                current_text.push_str(&text);
            }
            Ok(Event::CData(e)) => {
                current_text.push_str(&String::from_utf8_lossy(&e));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    root.ok_or_else(|| "Empty XML document".to_string())
}

fn add_to_object(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: serde_json::Value) {
    if let Some(existing) = obj.get_mut(key) {
        // Convert to array if needed
        match existing {
            serde_json::Value::Array(arr) => {
                arr.push(value);
            }
            _ => {
                let old = existing.take();
                *existing = serde_json::Value::Array(vec![old, value]);
            }
        }
    } else {
        obj.insert(key.to_string(), value);
    }
}

/// Parse XML and return a JSON representation
fn xml_parse(xml_str: String) -> Option<String> {
    xml_to_json_value(&xml_str).ok().map(|v| v.to_string())
}

/// Convert XML to JSON string
fn xml_to_json(xml_str: String) -> Option<String> {
    xml_to_json_value(&xml_str).ok().map(|v| v.to_string())
}

/// Check if XML is valid
fn xml_is_valid(xml_str: String) -> bool {
    let mut reader = Reader::from_str(&xml_str);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return true,
            Err(_) => return false,
            _ => {}
        }
    }
}

/// Extract text content from simple XML (strips tags)
fn xml_get_text(xml_str: String) -> String {
    let mut reader = Reader::from_str(&xml_str);
    reader.config_mut().trim_text(true);
    let mut text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Text(e)) => {
                let t = String::from_utf8_lossy(e.as_ref()).to_string();
                let t = t.trim();
                if !t.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(t);
                }
            }
            Ok(Event::CData(e)) => {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&String::from_utf8_lossy(&e));
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    text
}
