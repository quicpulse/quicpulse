//! Pretty output stream with syntax highlighting

use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// Cached syntax definitions - loaded once and reused for all highlighting
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Cached theme definitions - loaded once and reused for all highlighting
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Pretty stream with syntax highlighting
pub struct PrettyStream {
    /// Formatted output lines
    lines: Vec<String>,
    /// Current position
    position: usize,
}

impl PrettyStream {
    /// Create a new pretty stream with syntax highlighting
    pub fn new(content: &str, syntax_name: &str, theme_name: &str) -> Self {
        let lines = highlight_content(content, syntax_name, theme_name);
        Self {
            lines,
            position: 0,
        }
    }

    /// Create for JSON content
    pub fn json(content: &str, theme_name: &str) -> Self {
        Self::new(content, "JSON", theme_name)
    }

    /// Create for XML/HTML content
    pub fn xml(content: &str, theme_name: &str) -> Self {
        Self::new(content, "XML", theme_name)
    }

    /// Create for HTTP headers
    pub fn http(content: &str, theme_name: &str) -> Self {
        Self::new(content, "HTTP", theme_name)
    }

    /// Get all formatted content
    pub fn to_string(&self) -> String {
        self.lines.join("")
    }
}

impl Iterator for PrettyStream {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.lines.len() {
            return None;
        }

        let line = self.lines[self.position].clone();
        self.position += 1;
        Some(line)
    }
}

/// Highlight content with syntax highlighting
/// Uses cached SyntaxSet and ThemeSet for performance
fn highlight_content(content: &str, syntax_name: &str, theme_name: &str) -> Vec<String> {
    // Use cached syntax and theme sets
    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    // Find syntax by name or extension
    let syntax = ss.find_syntax_by_name(syntax_name)
        .or_else(|| ss.find_syntax_by_extension(syntax_name.to_lowercase().as_str()))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Find theme or use default
    let theme = ts.themes.get(theme_name)
        .or_else(|| ts.themes.get("base16-ocean.dark"))
        .unwrap_or_else(|| ts.themes.values().next().unwrap());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in LinesWithEndings::from(content) {
        match highlighter.highlight_line(line, ss) {
            Ok(ranges) => {
                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                result.push(escaped);
            }
            Err(_) => {
                // Fallback to plain text on error
                result.push(line.to_string());
            }
        }
    }

    // Reset terminal colors
    if !result.is_empty() {
        let last = result.len() - 1;
        result[last].push_str("\x1b[0m");
    }

    result
}

/// Buffered pretty stream that processes entire body first
pub struct BufferedPrettyStream {
    /// Complete formatted content
    content: String,
    /// Output as chunks
    chunks: Vec<String>,
    /// Current position
    position: usize,
}

/// Chunk size for buffered output (8KB - common default buffer)
pub const BUFFERED_CHUNK_SIZE: usize = 8 * 1024;

impl BufferedPrettyStream {
    /// Create from content with syntax highlighting
    pub fn new(content: &str, syntax_name: &str, theme_name: &str) -> Self {
        let pretty = PrettyStream::new(content, syntax_name, theme_name);
        let formatted = pretty.to_string();
        
        // Split into chunks
        let chunks: Vec<String> = formatted
            .as_bytes()
            .chunks(BUFFERED_CHUNK_SIZE)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect();

        Self {
            content: formatted,
            chunks,
            position: 0,
        }
    }

    /// Get the full content
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Iterator for BufferedPrettyStream {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.chunks.len() {
            return None;
        }

        let chunk = self.chunks[self.position].clone();
        self.position += 1;
        Some(chunk)
    }
}
