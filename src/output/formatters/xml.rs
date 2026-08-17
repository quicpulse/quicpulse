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
                if i + 8 < chars.len()
                    && chars[i + 1..i + 9].iter().collect::<String>() == "![CDATA["
                {
                    in_cdata = true;
                } else if i + 3 < chars.len()
                    && chars[i + 1] == '!'
                    && chars[i + 2] == '-'
                    && chars[i + 3] == '-'
                {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(xml: &str) -> String {
        format_xml(xml, &XmlFormatterOptions::default())
    }

    #[test]
    fn test_default_options() {
        assert_eq!(XmlFormatterOptions::default().indent, 2);
    }

    #[test]
    fn test_nests_child_elements() {
        assert_eq!(fmt("<a><b>text</b></a>"), "<a>\n  <b>text\n  </b>\n</a>");
    }

    #[test]
    fn test_declaration_is_not_indented_as_a_parent() {
        // <?xml ...?> must not increase depth, so <root> stays at column 0.
        assert_eq!(
            fmt("<?xml version=\"1.0\"?><root><item/></root>"),
            "<?xml version=\"1.0\"?>\n<root>\n  <item/>\n</root>"
        );
    }

    #[test]
    fn test_self_closing_tags_do_not_increase_depth() {
        assert_eq!(fmt("<a><b/><c/></a>"), "<a>\n  <b/>\n  <c/>\n</a>");
    }

    #[test]
    fn test_gt_inside_attribute_value_does_not_end_the_tag() {
        // The '>' in the title attribute must stay part of the same tag.
        assert_eq!(
            fmt("<a title=\"a > b\"><c/></a>"),
            "<a title=\"a > b\">\n  <c/>\n</a>"
        );
    }

    #[test]
    fn test_single_quoted_attribute_with_gt() {
        assert_eq!(fmt("<a t='x > y'/>"), "<a t='x > y'/>");
    }

    #[test]
    fn test_comment_with_gt_is_kept_intact() {
        assert_eq!(
            fmt("<a><!-- hi > there --><b/></a>"),
            "<a>\n  <!-- hi > there -->\n  <b/>\n</a>"
        );
    }

    #[test]
    fn test_cdata_with_gt_is_kept_intact() {
        assert_eq!(
            fmt("<a><![CDATA[ x > y ]]></a>"),
            "<a>\n  <![CDATA[ x > y ]]>\n</a>"
        );
    }

    #[test]
    fn test_existing_newlines_are_normalized() {
        // Pre-formatted input should not accumulate blank lines.
        assert_eq!(fmt("<a>\n<b/>\n</a>"), "<a>\n  <b/>\n</a>");
    }

    #[test]
    fn test_custom_indent_width() {
        assert_eq!(
            format_xml("<a><b/></a>", &XmlFormatterOptions { indent: 4 }),
            "<a>\n    <b/>\n</a>"
        );
        assert_eq!(
            format_xml("<a><b/></a>", &XmlFormatterOptions { indent: 0 }),
            "<a>\n<b/>\n</a>"
        );
    }

    #[test]
    fn test_unbalanced_closing_tags_do_not_panic() {
        // depth uses saturating_sub, so extra closers must not underflow.
        assert_eq!(fmt("</a></b>"), "</a>\n</b>");
    }

    #[test]
    fn test_empty_and_text_only_input() {
        assert_eq!(fmt(""), "");
        assert_eq!(fmt("hello"), "hello");
    }

    #[test]
    fn test_deeper_nesting_accumulates_indent() {
        let out = fmt("<a><b><c/></b></a>");
        assert_eq!(out, "<a>\n  <b>\n    <c/>\n  </b>\n</a>");
    }

    #[test]
    fn test_multibyte_content_is_preserved() {
        let out = fmt("<a><b>héllo → wörld</b></a>");
        assert!(out.contains("héllo → wörld"), "got: {out}");
    }

    #[test]
    fn test_doctype_does_not_increase_depth() {
        let out = fmt("<!DOCTYPE html><html><body/></html>");
        assert_eq!(out, "<!DOCTYPE html>\n<html>\n  <body/>\n</html>");
    }
}
