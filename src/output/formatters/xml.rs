//! XML formatting

/// XML formatting options
#[derive(Debug, Clone)]
pub struct XmlFormatterOptions {
    /// Indentation (default: 2 spaces)
    pub indent: usize,
}

impl Default for XmlFormatterOptions {
    fn default() -> Self {
        Self { indent: 2 }
    }
}

/// Format XML with indentation
///
/// This is a basic formatter that adds indentation based on tag nesting.
/// Properly handles '>' characters inside attribute values and comments.
pub fn format_xml(xml: &str, options: &XmlFormatterOptions) -> String {
    let indent_str = " ".repeat(options.indent);
    let mut result = String::new();
    let mut depth: usize = 0;
    let mut in_tag = false;
    let mut in_attribute = false;
    let mut in_comment = false;
    let mut in_cdata = false;
    let mut attribute_quote: Option<char> = None;
    let mut current_tag = String::new();
    let chars: Vec<char> = xml.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if in_cdata {
            current_tag.push(c);
            if current_tag.ends_with("]]>") {
                in_cdata = false;
                in_tag = false;
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                for _ in 0..depth {
                    result.push_str(&indent_str);
                }
                result.push_str(&current_tag);
            }
            i += 1;
            continue;
        }

        if in_comment {
            current_tag.push(c);
            if current_tag.ends_with("-->") {
                in_comment = false;
                in_tag = false;
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                for _ in 0..depth {
                    result.push_str(&indent_str);
                }
                result.push_str(&current_tag);
            }
            i += 1;
            continue;
        }

        match c {
            '<' if !in_attribute => {
                in_tag = true;
                current_tag.clear();
                current_tag.push(c);
                if i + 8 < chars.len() && chars[i+1..i+9].iter().collect::<String>() == "![CDATA[" {
                    in_cdata = true;
                } else if i + 3 < chars.len() && chars[i + 1] == '!' && chars[i + 2] == '-' && chars[i + 3] == '-' {
                    in_comment = true;
                }
            }
            '>' if !in_attribute && in_tag => {
                current_tag.push(c);
                in_tag = false;

                let is_closing = current_tag.starts_with("</");
                let is_self_closing = current_tag.ends_with("/>");
                let is_declaration = current_tag.starts_with("<?") || current_tag.starts_with("<!");

                if is_closing {
                    depth = depth.saturating_sub(1);
                }

                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                for _ in 0..depth {
                    result.push_str(&indent_str);
                }
                result.push_str(&current_tag);

                if !is_closing && !is_self_closing && !is_declaration {
                    depth += 1;
                }
            }
            '"' | '\'' if in_tag => {
                current_tag.push(c);
                if let Some(quote) = attribute_quote {
                    if quote == c {
                        attribute_quote = None;
                        in_attribute = false;
                    }
                } else {
                    attribute_quote = Some(c);
                    in_attribute = true;
                }
            }
            _ if in_tag => {
                current_tag.push(c);
            }
            _ => {
                let trimmed = c.to_string();
                if !trimmed.trim().is_empty() || !result.ends_with('>') {
                    result.push(c);
                }
            }
        }
        i += 1;
    }

    result
}
