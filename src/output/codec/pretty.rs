//! Pretty printing codec with lazy syntax highlighting
//!
//! Applies syntax highlighting to decoded text on demand.

use bytes::BytesMut;
use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
use tokio_util::codec::Decoder;

use crate::output::error::StreamError;

/// Cached syntax definitions
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Cached theme definitions
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Codec that applies syntax highlighting to text
pub struct PrettyCodec {
    /// Syntax name for highlighting
    syntax_name: String,
    /// Theme name for colors
    theme_name: String,
    /// Buffer for incomplete lines
    buffer: String,
    /// Whether we've finished processing
    finished: bool,
}

impl PrettyCodec {
    /// Create a new pretty codec
    pub fn new(syntax_name: impl Into<String>, theme_name: impl Into<String>) -> Self {
        Self {
            syntax_name: syntax_name.into(),
            theme_name: theme_name.into(),
            buffer: String::new(),
            finished: false,
        }
    }

    /// Create for JSON content
    pub fn json(theme_name: impl Into<String>) -> Self {
        Self::new("JSON", theme_name)
    }

    /// Create for XML content
    pub fn xml(theme_name: impl Into<String>) -> Self {
        Self::new("XML", theme_name)
    }

    /// Create for HTTP headers
    pub fn http(theme_name: impl Into<String>) -> Self {
        Self::new("HTTP", theme_name)
    }

    /// Highlight a single line
    fn highlight_line(&self, line: &str) -> Result<String, StreamError> {
        let ss = &*SYNTAX_SET;
        let ts = &*THEME_SET;

        let syntax = ss.find_syntax_by_name(&self.syntax_name)
            .or_else(|| ss.find_syntax_by_extension(&self.syntax_name.to_lowercase()))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let theme = ts.themes.get(&self.theme_name)
            .or_else(|| ts.themes.get("base16-ocean.dark"))
            .ok_or_else(|| StreamError::highlight("No theme available"))?;

        let mut highlighter = HighlightLines::new(syntax, theme);

        match highlighter.highlight_line(line, ss) {
            Ok(ranges) => {
                let mut escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                // Reset colors at end of line
                escaped.push_str("\x1b[0m");
                Ok(escaped)
            }
            Err(e) => Err(StreamError::highlight(e.to_string())),
        }
    }

    /// Highlight multiple lines
    fn highlight_lines(&self, text: &str) -> Result<String, StreamError> {
        let ss = &*SYNTAX_SET;
        let ts = &*THEME_SET;

        let syntax = ss.find_syntax_by_name(&self.syntax_name)
            .or_else(|| ss.find_syntax_by_extension(&self.syntax_name.to_lowercase()))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let theme = ts.themes.get(&self.theme_name)
            .or_else(|| ts.themes.get("base16-ocean.dark"))
            .ok_or_else(|| StreamError::highlight("No theme available"))?;

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = String::new();

        for line in text.lines() {
            match highlighter.highlight_line(line, ss) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    result.push_str(&escaped);
                    result.push('\n');
                }
                Err(_) => {
                    // Fallback to plain text
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        // Reset colors at end
        if !result.is_empty() {
            result.push_str("\x1b[0m");
        }

        Ok(result)
    }
}

impl Decoder for PrettyCodec {
    type Item = String;
    type Error = StreamError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // Decode bytes to text
        let text = String::from_utf8_lossy(src).to_string();
        src.clear();

        // Combine with buffer
        let full_text = if self.buffer.is_empty() {
            text
        } else {
            let mut combined = std::mem::take(&mut self.buffer);
            combined.push_str(&text);
            combined
        };

        // Find complete lines
        if let Some(newline_pos) = full_text.rfind('\n') {
            let complete = &full_text[..=newline_pos];
            self.buffer = full_text[newline_pos + 1..].to_string();

            // Highlight the complete lines
            let highlighted = self.highlight_lines(complete)?;
            Ok(Some(highlighted))
        } else {
            // No complete line yet
            self.buffer = full_text;
            Ok(None)
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Process remaining bytes
        if !src.is_empty() {
            let text = String::from_utf8_lossy(src).to_string();
            src.clear();
            self.buffer.push_str(&text);
        }

        // Return remaining buffer
        if !self.buffer.is_empty() && !self.finished {
            self.finished = true;
            let text = std::mem::take(&mut self.buffer);
            let highlighted = self.highlight_lines(&text)?;
            Ok(Some(highlighted))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_json() {
        let codec = PrettyCodec::json("base16-ocean.dark");
        let result = codec.highlight_line("{\"key\": \"value\"}\n");
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        // Should contain ANSI escape codes
        assert!(highlighted.contains("\x1b["));
    }
}
