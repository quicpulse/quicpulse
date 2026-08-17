use once_cell::sync::Lazy;
use regex::Regex;

mod pie_colors {
    pub const GREY: u8 = 102; // #7D7D7D -> 102
    pub const AQUA: u8 = 109; // #7A9EB5 -> 109
    pub const PURPLE: u8 = 134; // #9E54D6 -> 134
    pub const ORANGE: u8 = 208; // #F2913D -> 208
    pub const RED: u8 = 167; // #E34F45 -> 167
    pub const BLUE: u8 = 68; // #426BD1 -> 68
    pub const PINK: u8 = 176; // #DE85DE -> 176
    pub const GREEN: u8 = 71; // #63C27A -> 71
    pub const YELLOW: u8 = 185; // #CCCC3D -> 185
    pub const WHITE: u8 = 255; // #F5F5F0 -> 255
    pub const PRIMARY: u8 = 250; // bright white for dark terminals
}

/// ANSI escape code helpers
mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";

    #[inline]
    pub fn fg256(color: u8) -> String {
        format!("\x1b[38;5;{}m", color)
    }

    #[inline]
    pub fn bold_fg256(color: u8) -> String {
        format!("\x1b[1;38;5;{}m", color)
    }
}

/// Regex patterns for parsing HTTP headers
static HTTP_REQUEST_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([A-Z]+)( +)([^ ]+)( +)(HTTP)(/)(\d+\.?\d*)$").unwrap());

static HTTP_RESPONSE_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(HTTP)(/)([\d.]+)( +)(\d{3})( ?)(.*)$").unwrap());

static HTTP_HEADER_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([^:]+)(:)(.*)$").unwrap());

/// Available color styles
#[derive(Debug, Clone, PartialEq)]
pub enum ColorStyle {
    Auto,
    PieDark,
    PieLight,
    /// Solarized dark (legacy)
    SolarizedDark,
    /// Solarized light (legacy)
    SolarizedLight,
    /// Monokai (legacy)
    Monokai,
    /// Custom theme name (falls back to pie)
    Custom(String),
}

impl ColorStyle {
    /// Parse style name
    #[inline]
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "auto" | "pie" | "pie-dark" => ColorStyle::Auto,
            "pie-light" => ColorStyle::PieLight,
            "solarized" | "solarized-dark" | "solarized256" => ColorStyle::SolarizedDark,
            "solarized-light" => ColorStyle::SolarizedLight,
            "monokai" => ColorStyle::Monokai,
            other => ColorStyle::Custom(other.to_string()),
        }
    }

    /// List all available built-in styles
    pub fn list_styles() -> Vec<(&'static str, &'static str)> {
        vec![
            ("auto", "compatible style (default)"),
            ("pie", "pie style (same as auto)"),
            ("pie-dark", "pie dark style"),
            ("pie-light", "pie light style"),
            ("solarized-dark", "Solarized Dark theme"),
            ("solarized-light", "Solarized Light theme"),
            ("monokai", "Monokai theme"),
        ]
    }
}

impl Default for ColorStyle {
    #[inline]
    fn default() -> Self {
        ColorStyle::Auto
    }
}

pub struct ColorFormatter {
    #[allow(dead_code)]
    style: ColorStyle,
}

impl ColorFormatter {
    /// Create a new color formatter
    pub fn new(style: ColorStyle) -> Self {
        Self { style }
    }

    pub fn format_headers(&self, headers: &str) -> String {
        let mut result = String::with_capacity(headers.len() * 2);

        for line in headers.lines() {
            if line.is_empty() {
                result.push('\n');
                continue;
            }

            // Try to match request line: METHOD /path HTTP/1.1
            if let Some(caps) = HTTP_REQUEST_LINE.captures(line) {
                let method = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let space1 = caps.get(2).map(|m| m.as_str()).unwrap_or(" ");
                let path = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let space2 = caps.get(4).map(|m| m.as_str()).unwrap_or(" ");
                let http = caps.get(5).map(|m| m.as_str()).unwrap_or("HTTP");
                let slash = caps.get(6).map(|m| m.as_str()).unwrap_or("/");
                let version = caps.get(7).map(|m| m.as_str()).unwrap_or("");

                let method_color = match method {
                    "GET" | "HEAD" | "OPTIONS" => pie_colors::GREEN,
                    "POST" => pie_colors::YELLOW,
                    "PUT" | "PATCH" => pie_colors::ORANGE,
                    "DELETE" => pie_colors::RED,
                    _ => pie_colors::GREY,
                };

                result.push_str(&ansi::bold_fg256(method_color));
                result.push_str(method);
                result.push_str(ansi::RESET);
                result.push_str(space1);

                // Path in primary color (bold)
                result.push_str(&ansi::bold_fg256(pie_colors::PRIMARY));
                result.push_str(path);
                result.push_str(ansi::RESET);
                result.push_str(space2);

                // HTTP/version in grey
                result.push_str(&ansi::bold_fg256(pie_colors::GREY));
                result.push_str(http);
                result.push_str(slash);
                result.push_str(version);
                result.push_str(ansi::RESET);
                result.push('\n');
                continue;
            }

            // Try to match response line: HTTP/1.1 200 OK
            if let Some(caps) = HTTP_RESPONSE_LINE.captures(line) {
                let http = caps.get(1).map(|m| m.as_str()).unwrap_or("HTTP");
                let slash = caps.get(2).map(|m| m.as_str()).unwrap_or("/");
                let version = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let space = caps.get(4).map(|m| m.as_str()).unwrap_or(" ");
                let status = caps.get(5).map(|m| m.as_str()).unwrap_or("");
                let space2 = caps.get(6).map(|m| m.as_str()).unwrap_or(" ");
                let reason = caps.get(7).map(|m| m.as_str()).unwrap_or("");

                // HTTP/version in grey
                result.push_str(&ansi::bold_fg256(pie_colors::GREY));
                result.push_str(http);
                result.push_str(slash);
                result.push_str(version);
                result.push_str(ansi::RESET);
                result.push_str(space);

                let status_color = match status.chars().next() {
                    Some('1') => pie_colors::AQUA,   // 1xx Informational
                    Some('2') => pie_colors::GREEN,  // 2xx Success
                    Some('3') => pie_colors::YELLOW, // 3xx Redirect
                    Some('4') => pie_colors::ORANGE, // 4xx Client Error
                    Some('5') => pie_colors::RED,    // 5xx Server Error
                    _ => pie_colors::GREY,
                };

                result.push_str(&ansi::bold_fg256(status_color));
                result.push_str(status);
                result.push_str(space2);
                result.push_str(reason);
                result.push_str(ansi::RESET);
                result.push('\n');
                continue;
            }

            // Try to match header line: Name: Value
            if let Some(caps) = HTTP_HEADER_LINE.captures(line) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let colon = caps.get(2).map(|m| m.as_str()).unwrap_or(":");
                let value = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                // Header name in blue
                result.push_str(&ansi::fg256(pie_colors::BLUE));
                result.push_str(name);
                result.push_str(ansi::RESET);

                // Colon in grey
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push_str(colon);
                result.push_str(ansi::RESET);

                // Value in primary (white)
                result.push_str(&ansi::fg256(pie_colors::PRIMARY));
                result.push_str(value);
                result.push_str(ansi::RESET);
                result.push('\n');
                continue;
            }

            // Fallback: just output the line
            result.push_str(line);
            result.push('\n');
        }

        // Final reset
        result.push_str(ansi::RESET);
        result
    }

    pub fn format_json(&self, json: &str) -> String {
        colorize_json(json)
    }

    /// Format XML/HTML body
    pub fn format_xml(&self, xml: &str) -> String {
        colorize_xml(xml)
    }

    /// Format plain text (no highlighting)
    pub fn format_plain(&self, text: &str) -> String {
        text.to_string()
    }

    /// Format content based on MIME type
    pub fn format_by_mime(&self, content: &str, mime_type: &str) -> String {
        let base_mime = mime_type.split(';').next().unwrap_or(mime_type).trim();

        match base_mime {
            "application/json" | "text/json" => self.format_json(content),
            m if m.ends_with("+json") => self.format_json(content),
            "application/xml" | "text/xml" => self.format_xml(content),
            "text/html" | "application/xhtml+xml" => self.format_xml(content),
            m if m.ends_with("+xml") => self.format_xml(content),
            _ => self.format_plain(content),
        }
    }
}

fn colorize_json(json: &str) -> String {
    let mut result = String::with_capacity(json.len() * 2);
    let mut chars = json.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;
    // Stack to track context: true = object (expect keys), false = array (expect values)
    let mut context_stack: Vec<bool> = Vec::new();
    let mut expect_key = false; // Whether next string should be a key

    while let Some(c) = chars.next() {
        if escape_next {
            result.push(c);
            escape_next = false;
            continue;
        }

        if c == '\\' && in_string {
            result.push(c);
            escape_next = true;
            continue;
        }

        if c == '"' {
            if in_string {
                // End of string
                result.push(c);
                result.push_str(ansi::RESET);
                in_string = false;
            } else {
                // Start of string
                in_string = true;
                let color = if expect_key {
                    pie_colors::PINK // Keys in pink
                } else {
                    pie_colors::GREEN // String values in green
                };
                result.push_str(&ansi::fg256(color));
                result.push(c);
            }
            continue;
        }

        if in_string {
            result.push(c);
            continue;
        }

        // Not in string - check for special tokens
        match c {
            '{' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                context_stack.push(true); // Entering object
                expect_key = true; // First thing in object is a key
            }
            '[' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                context_stack.push(false); // Entering array
                expect_key = false; // Arrays contain values, not keys
            }
            '}' | ']' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                context_stack.pop();
                // After closing, check parent context for next expectation
                expect_key = false;
            }
            ',' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                // After comma, check context: in object expect key, in array expect value
                expect_key = context_stack.last().copied().unwrap_or(false);
            }
            ':' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                expect_key = false; // After colon, we're parsing a value
            }
            't' => {
                // Check for 'true'
                if chars.peek() == Some(&'r') {
                    let rest: String = chars.by_ref().take(3).collect();
                    if rest == "rue" {
                        result.push_str(&ansi::fg256(pie_colors::ORANGE));
                        result.push_str("true");
                        result.push_str(ansi::RESET);
                        continue;
                    }
                    result.push(c);
                    result.push_str(&rest);
                } else {
                    result.push(c);
                }
            }
            'f' => {
                // Check for 'false'
                if chars.peek() == Some(&'a') {
                    let rest: String = chars.by_ref().take(4).collect();
                    if rest == "alse" {
                        result.push_str(&ansi::fg256(pie_colors::ORANGE));
                        result.push_str("false");
                        result.push_str(ansi::RESET);
                        continue;
                    }
                    result.push(c);
                    result.push_str(&rest);
                } else {
                    result.push(c);
                }
            }
            'n' => {
                // Check for 'null'
                if chars.peek() == Some(&'u') {
                    let rest: String = chars.by_ref().take(3).collect();
                    if rest == "ull" {
                        result.push_str(&ansi::fg256(pie_colors::ORANGE));
                        result.push_str("null");
                        result.push_str(ansi::RESET);
                        continue;
                    }
                    result.push(c);
                    result.push_str(&rest);
                } else {
                    result.push(c);
                }
            }
            '0'..='9' | '-' => {
                // Number
                result.push_str(&ansi::fg256(pie_colors::AQUA));
                result.push(c);
                // Consume rest of number
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit()
                        || next == '.'
                        || next == 'e'
                        || next == 'E'
                        || next == '+'
                        || next == '-'
                    {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(ansi::RESET);
            }
            ' ' | '\n' | '\r' | '\t' => {
                result.push(c);
            }
            _ => {
                result.push(c);
            }
        }
    }

    result.push_str(ansi::RESET);
    result
}

/// Colorize XML/HTML with basic syntax highlighting
fn colorize_xml(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len() * 2);
    let mut chars = xml.chars().peekable();
    let mut in_tag = false;
    let mut in_string = false;
    let mut string_char = '"';

    while let Some(c) = chars.next() {
        if in_string {
            result.push(c);
            if c == string_char {
                result.push_str(ansi::RESET);
                in_string = false;
            }
            continue;
        }

        match c {
            '<' => {
                in_tag = true;
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                // Check for tag name
                let mut tag_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric()
                        || next == '/'
                        || next == '!'
                        || next == '?'
                        || next == '-'
                    {
                        tag_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if !tag_name.is_empty() {
                    result.push_str(ansi::RESET);
                    result.push_str(&ansi::fg256(pie_colors::BLUE));
                    result.push_str(&tag_name);
                    result.push_str(ansi::RESET);
                }
            }
            '>' => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
                in_tag = false;
            }
            '"' | '\'' if in_tag => {
                string_char = c;
                in_string = true;
                result.push_str(&ansi::fg256(pie_colors::GREEN));
                result.push(c);
            }
            '=' if in_tag => {
                result.push_str(&ansi::fg256(pie_colors::GREY));
                result.push(c);
                result.push_str(ansi::RESET);
            }
            _ if in_tag && (c.is_alphabetic() || c == '-' || c == '_') => {
                // Attribute name
                result.push_str(&ansi::fg256(pie_colors::PINK));
                result.push(c);
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '-' || next == '_' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(ansi::RESET);
            }
            _ => {
                result.push(c);
            }
        }
    }

    result.push_str(ansi::RESET);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_request_line() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let input = "GET /api/users HTTP/1.1\n";
        let result = formatter.format_headers(input);

        // Should contain color codes
        assert!(result.contains("\x1b["));
        // Should contain the original text
        assert!(result.contains("GET"));
        assert!(result.contains("/api/users"));
        assert!(result.contains("HTTP/1.1"));
    }

    #[test]
    fn test_format_response_line() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let input = "HTTP/1.1 200 OK\n";
        let result = formatter.format_headers(input);

        assert!(result.contains("\x1b["));
        assert!(result.contains("HTTP/1.1"));
        assert!(result.contains("200"));
        assert!(result.contains("OK"));
    }

    #[test]
    fn test_format_header_line() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let input = "Content-Type: application/json\n";
        let result = formatter.format_headers(input);

        assert!(result.contains("\x1b["));
        assert!(result.contains("Content-Type"));
        assert!(result.contains("application/json"));
    }

    #[test]
    fn test_format_json() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let input = r#"{"name": "test", "value": 123, "active": true}"#;
        let result = formatter.format_json(input);

        assert!(result.contains("\x1b["));
        assert!(result.contains("name"));
        assert!(result.contains("test"));
        assert!(result.contains("123"));
        assert!(result.contains("true"));
    }

    #[test]
    fn test_status_code_colors() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);

        // 2xx should be green
        let r200 = formatter.format_headers("HTTP/1.1 200 OK\n");
        assert!(r200.contains(&format!("\x1b[1;38;5;{}m", pie_colors::GREEN)));

        // 4xx should be orange
        let r404 = formatter.format_headers("HTTP/1.1 404 Not Found\n");
        assert!(r404.contains(&format!("\x1b[1;38;5;{}m", pie_colors::ORANGE)));

        // 5xx should be red
        let r500 = formatter.format_headers("HTTP/1.1 500 Server Error\n");
        assert!(r500.contains(&format!("\x1b[1;38;5;{}m", pie_colors::RED)));
    }

    #[test]
    fn test_method_colors() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);

        // GET should be green
        let get = formatter.format_headers("GET / HTTP/1.1\n");
        assert!(get.contains(&format!("\x1b[1;38;5;{}m", pie_colors::GREEN)));

        // POST should be yellow
        let post = formatter.format_headers("POST / HTTP/1.1\n");
        assert!(post.contains(&format!("\x1b[1;38;5;{}m", pie_colors::YELLOW)));

        // DELETE should be red
        let delete = formatter.format_headers("DELETE / HTTP/1.1\n");
        assert!(delete.contains(&format!("\x1b[1;38;5;{}m", pie_colors::RED)));
    }

    // ---- ColorStyle ----

    #[test]
    fn test_color_style_parse_known_names() {
        assert_eq!(ColorStyle::parse("auto"), ColorStyle::Auto);
        assert_eq!(ColorStyle::parse("pie"), ColorStyle::Auto);
        assert_eq!(ColorStyle::parse("pie-dark"), ColorStyle::Auto);
        assert_eq!(ColorStyle::parse("pie-light"), ColorStyle::PieLight);
        assert_eq!(ColorStyle::parse("solarized"), ColorStyle::SolarizedDark);
        assert_eq!(
            ColorStyle::parse("solarized-dark"),
            ColorStyle::SolarizedDark
        );
        assert_eq!(ColorStyle::parse("solarized256"), ColorStyle::SolarizedDark);
        assert_eq!(
            ColorStyle::parse("solarized-light"),
            ColorStyle::SolarizedLight
        );
        assert_eq!(ColorStyle::parse("monokai"), ColorStyle::Monokai);
    }

    #[test]
    fn test_color_style_parse_is_case_insensitive() {
        assert_eq!(ColorStyle::parse("MONOKAI"), ColorStyle::Monokai);
        assert_eq!(ColorStyle::parse("Pie-Light"), ColorStyle::PieLight);
    }

    #[test]
    fn test_color_style_parse_unknown_becomes_custom_lowercased() {
        assert_eq!(
            ColorStyle::parse("Dracula"),
            ColorStyle::Custom("dracula".to_string())
        );
    }

    #[test]
    fn test_color_style_default_is_auto() {
        assert_eq!(ColorStyle::default(), ColorStyle::Auto);
    }

    #[test]
    fn test_list_styles_covers_every_documented_name() {
        let styles = ColorStyle::list_styles();
        assert!(!styles.is_empty());
        let names: Vec<&str> = styles.iter().map(|(n, _)| *n).collect();
        for expected in [
            "auto",
            "pie",
            "pie-dark",
            "pie-light",
            "solarized-dark",
            "solarized-light",
            "monokai",
        ] {
            assert!(names.contains(&expected), "missing style {expected}");
        }
        // Every entry must carry a description.
        assert!(styles.iter().all(|(_, d)| !d.is_empty()));
    }

    // ---- format_headers edge cases ----

    #[test]
    fn test_blank_line_separating_headers_from_body() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let out = formatter.format_headers("Host: x\n\n");
        assert!(out.contains("Host"));
    }

    #[test]
    fn test_unrecognized_line_passes_through() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let out = formatter.format_headers("not a header or request line\n");
        assert!(out.contains("not a header or request line"));
    }

    #[test]
    fn test_informational_and_redirect_status_colors() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let r100 = formatter.format_headers("HTTP/1.1 100 Continue\n");
        assert!(r100.contains(&format!("\x1b[1;38;5;{}m", pie_colors::AQUA)));

        let r301 = formatter.format_headers("HTTP/1.1 301 Moved Permanently\n");
        assert!(r301.contains(&format!("\x1b[1;38;5;{}m", pie_colors::YELLOW)));
    }

    #[test]
    fn test_put_and_unknown_method_colors() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let put = formatter.format_headers("PUT / HTTP/1.1\n");
        assert!(put.contains(&format!("\x1b[1;38;5;{}m", pie_colors::ORANGE)));

        // An unrecognized (but uppercase) verb falls back to grey.
        let custom = formatter.format_headers("PURGE / HTTP/1.1\n");
        assert!(custom.contains(&format!("\x1b[1;38;5;{}m", pie_colors::GREY)));
    }

    #[test]
    fn test_response_line_without_reason_phrase() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let out = formatter.format_headers("HTTP/2 204\n");
        assert!(out.contains("204"));
        assert!(out.contains(&format!("\x1b[1;38;5;{}m", pie_colors::GREEN)));
    }

    #[test]
    fn test_header_name_and_value_get_distinct_colors() {
        let formatter = ColorFormatter::new(ColorStyle::Auto);
        let out = formatter.format_headers("Accept: */*\n");
        assert!(out.contains(&format!("\x1b[38;5;{}m", pie_colors::BLUE)));
        assert!(out.contains(&format!("\x1b[38;5;{}m", pie_colors::PRIMARY)));
    }

    // ---- JSON colorization ----

    #[test]
    fn test_json_keys_and_values_get_different_colors() {
        let out = colorize_json(r#"{"k":"v"}"#);
        assert!(
            out.contains(&format!("\x1b[38;5;{}m", pie_colors::PINK)),
            "key not pink"
        );
        assert!(
            out.contains(&format!("\x1b[38;5;{}m", pie_colors::GREEN)),
            "value not green"
        );
    }

    #[test]
    fn test_json_array_elements_are_values_not_keys() {
        // Strings inside an array must be coloured as values, never as keys.
        let out = colorize_json(r#"["a","b"]"#);
        assert!(!out.contains(&format!("\x1b[38;5;{}m", pie_colors::PINK)));
        assert!(out.contains(&format!("\x1b[38;5;{}m", pie_colors::GREEN)));
    }

    #[test]
    fn test_json_literals_are_coloured() {
        for literal in ["true", "false", "null"] {
            let out = colorize_json(&format!(r#"{{"k":{literal}}}"#));
            assert!(out.contains(literal), "{literal} missing from output");
            assert!(
                out.contains(&format!("\x1b[38;5;{}m", pie_colors::ORANGE)),
                "{literal} not coloured"
            );
        }
    }

    #[test]
    fn test_json_numbers_including_exponents_and_negatives() {
        let out = colorize_json(r#"{"a":-1.5e-3,"b":42}"#);
        assert!(out.contains("-1.5e-3"), "exponent number mangled: {out}");
        assert!(out.contains("42"));
        assert!(out.contains(&format!("\x1b[38;5;{}m", pie_colors::AQUA)));
    }

    #[test]
    fn test_json_escaped_quote_inside_string_does_not_end_the_string() {
        let out = colorize_json(r#"{"k":"a\"b"}"#);
        // The escaped quote must survive, and the trailing } must still be present.
        assert!(out.contains(r#"a\"b"#), "escape mangled: {out}");
        assert!(out.ends_with(ansi::RESET));
    }

    #[test]
    fn test_json_colorize_preserves_all_plain_characters() {
        // Stripping ANSI codes must give back the original text.
        let src = r#"{"a": [1, true, null, "x"], "b": {"c": -2.5}}"#;
        let out = colorize_json(src);
        assert_eq!(strip_ansi(&out), src);
    }

    #[test]
    fn test_json_colorize_preserves_whitespace_and_newlines() {
        let src = "{\n  \"a\": 1\n}";
        assert_eq!(strip_ansi(&colorize_json(src)), src);
    }

    #[test]
    fn test_json_words_starting_with_literal_prefixes_are_not_mangled() {
        // Bare words like `nope` / `text` / `fine` are not JSON literals; the
        // colorizer must still emit their characters unchanged.
        for word in ["nope", "text", "fine"] {
            let src = format!(r#"{{"k":{word}}}"#);
            assert_eq!(strip_ansi(&colorize_json(&src)), src, "mangled {word}");
        }
    }

    #[test]
    fn test_json_nested_context_switches_back_to_keys_after_array() {
        let src = r#"{"a":[1],"b":2}"#;
        assert_eq!(strip_ansi(&colorize_json(src)), src);
    }

    // ---- XML colorization ----

    #[test]
    fn test_xml_tag_names_and_attributes_are_coloured() {
        let out = colorize_xml(r#"<a href="x">t</a>"#);
        assert!(
            out.contains(&format!("\x1b[38;5;{}m", pie_colors::BLUE)),
            "tag not blue"
        );
        assert!(
            out.contains(&format!("\x1b[38;5;{}m", pie_colors::PINK)),
            "attr not pink"
        );
        assert!(
            out.contains(&format!("\x1b[38;5;{}m", pie_colors::GREEN)),
            "attr value not green"
        );
    }

    #[test]
    fn test_xml_colorize_preserves_all_plain_characters() {
        let src = r#"<root><item id="1" data-x='y'>text</item><self/></root>"#;
        assert_eq!(strip_ansi(&colorize_xml(src)), src);
    }

    #[test]
    fn test_xml_single_quoted_attribute_values() {
        let src = "<a t='v'/>";
        assert_eq!(strip_ansi(&colorize_xml(src)), src);
    }

    #[test]
    fn test_xml_declaration_and_comment_survive() {
        let src = "<?xml version=\"1.0\"?><!-- c --><a/>";
        assert_eq!(strip_ansi(&colorize_xml(src)), src);
    }

    #[test]
    fn test_xml_text_outside_tags_is_untouched() {
        let src = "<a>plain text 123</a>";
        assert_eq!(strip_ansi(&colorize_xml(src)), src);
    }

    // ---- MIME dispatch ----

    #[test]
    fn test_format_by_mime_json_variants() {
        let f = ColorFormatter::new(ColorStyle::Auto);
        let body = r#"{"k":1}"#;
        for mime in [
            "application/json",
            "text/json",
            "application/vnd.api+json",
            "application/json; charset=utf-8",
        ] {
            let out = f.format_by_mime(body, mime);
            assert_eq!(out, f.format_json(body), "mime {mime} not routed to JSON");
        }
    }

    #[test]
    fn test_format_by_mime_xml_and_html_variants() {
        let f = ColorFormatter::new(ColorStyle::Auto);
        let body = "<a/>";
        for mime in [
            "application/xml",
            "text/xml",
            "text/html",
            "application/xhtml+xml",
            "image/svg+xml",
            "text/html; charset=utf-8",
        ] {
            let out = f.format_by_mime(body, mime);
            assert_eq!(out, f.format_xml(body), "mime {mime} not routed to XML");
        }
    }

    #[test]
    fn test_format_by_mime_falls_back_to_plain() {
        let f = ColorFormatter::new(ColorStyle::Auto);
        assert_eq!(f.format_by_mime("hello", "text/plain"), "hello");
        assert_eq!(
            f.format_by_mime("hello", "application/octet-stream"),
            "hello"
        );
    }

    #[test]
    fn test_format_plain_is_identity() {
        let f = ColorFormatter::new(ColorStyle::Auto);
        assert_eq!(f.format_plain("a\nb\t<>{}"), "a\nb\t<>{}");
    }

    #[test]
    fn test_formatter_accepts_every_style_variant() {
        for style in [
            ColorStyle::Auto,
            ColorStyle::PieDark,
            ColorStyle::PieLight,
            ColorStyle::SolarizedDark,
            ColorStyle::SolarizedLight,
            ColorStyle::Monokai,
            ColorStyle::Custom("whatever".to_string()),
        ] {
            let f = ColorFormatter::new(style);
            assert!(f.format_headers("HTTP/1.1 200 OK\n").contains("200"));
        }
    }

    /// Remove ANSI SGR escape sequences so tests can assert on plain content.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip "[...m"
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }
}
