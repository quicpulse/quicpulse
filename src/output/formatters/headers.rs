//! HTTP headers formatting

/// Headers formatting options
#[derive(Debug, Clone)]
pub struct HeadersFormatterOptions {
    /// Sort headers alphabetically
    pub sort: bool,
}

impl Default for HeadersFormatterOptions {
    fn default() -> Self {
        Self { sort: false }
    }
}

/// Format HTTP headers
pub fn format_headers(headers: &str, options: &HeadersFormatterOptions) -> String {
    if !options.sort {
        return headers.to_string();
    }

    // Split headers (skip status line)
    let mut lines: Vec<&str> = headers.lines().collect();
    
    if lines.is_empty() {
        return headers.to_string();
    }

    // Keep first line (status) separate, sort the rest
    let first_line = lines.remove(0);
    lines.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    let mut result = String::from(first_line);
    result.push('\n');
    
    for line in lines {
        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Parse a header line into (name, value)
pub fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let colon_pos = line.find(':')?;
    let name = &line[..colon_pos];
    let value = line[colon_pos + 1..].trim();
    Some((name, value))
}
