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

        let syntax = ss
            .find_syntax_by_name(&self.syntax_name)
            .or_else(|| ss.find_syntax_by_extension(&self.syntax_name.to_lowercase()))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let theme = ts
            .themes
            .get(&self.theme_name)
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

        let syntax = ss
            .find_syntax_by_name(&self.syntax_name)
            .or_else(|| ss.find_syntax_by_extension(&self.syntax_name.to_lowercase()))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let theme = ts
            .themes
            .get(&self.theme_name)
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

    /// Remove ANSI SGR sequences so tests can compare plain content.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
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

    #[test]
    fn test_highlight_json() {
        let codec = PrettyCodec::json("base16-ocean.dark");
        let result = codec.highlight_line("{\"key\": \"value\"}\n");
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        // Should contain ANSI escape codes
        assert!(highlighted.contains("\x1b["));
    }

    #[test]
    fn test_highlight_line_always_resets_colour() {
        let codec = PrettyCodec::json("base16-ocean.dark");
        let out = codec.highlight_line("{\"a\": 1}").unwrap();
        assert!(out.ends_with("\x1b[0m"), "line must end with a reset");
    }

    #[test]
    fn test_named_constructors_select_their_syntax() {
        assert_eq!(PrettyCodec::json("t").syntax_name, "JSON");
        assert_eq!(PrettyCodec::xml("t").syntax_name, "XML");
        assert_eq!(PrettyCodec::http("t").syntax_name, "HTTP");
        assert_eq!(PrettyCodec::json("my-theme").theme_name, "my-theme");
    }

    #[test]
    fn test_new_starts_empty_and_unfinished() {
        let codec = PrettyCodec::new("JSON", "base16-ocean.dark");
        assert!(codec.buffer.is_empty());
        assert!(!codec.finished);
    }

    #[test]
    fn test_unknown_theme_falls_back_instead_of_failing() {
        let codec = PrettyCodec::json("no-such-theme-xyz");
        let out = codec.highlight_line("{\"a\": 1}").unwrap();
        assert!(out.contains("\x1b["));
    }

    #[test]
    fn test_unknown_syntax_falls_back_to_plain_text() {
        let codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");
        let out = codec.highlight_line("hello world").unwrap();
        assert_eq!(strip_ansi(&out), "hello world");
    }

    #[test]
    fn test_syntax_resolved_by_extension_when_name_misses() {
        // "json" is not a syntax *name* ("JSON" is), so this exercises the
        // find_syntax_by_extension fallback.
        let codec = PrettyCodec::new("json", "base16-ocean.dark");
        let out = codec.highlight_line("{\"a\": 1}").unwrap();
        assert!(out.contains("\x1b["));
    }

    #[test]
    fn test_highlight_lines_preserves_content_and_line_count() {
        let codec = PrettyCodec::json("base16-ocean.dark");
        let out = codec.highlight_lines("{\n\"a\": 1\n}\n").unwrap();
        assert_eq!(strip_ansi(&out), "{\n\"a\": 1\n}\n");
    }

    #[test]
    fn test_highlight_lines_on_empty_input_stays_empty() {
        let codec = PrettyCodec::json("base16-ocean.dark");
        assert_eq!(codec.highlight_lines("").unwrap(), "");
    }

    // ---- Decoder behaviour ----

    #[test]
    fn test_decode_empty_buffer_yields_nothing() {
        let mut codec = PrettyCodec::json("base16-ocean.dark");
        let mut buf = BytesMut::new();
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }

    #[test]
    fn test_decode_withholds_incomplete_line() {
        let mut codec = PrettyCodec::json("base16-ocean.dark");
        let mut buf = BytesMut::from(&b"{\"a\": "[..]);

        // No newline yet, so nothing is emitted and the input is buffered.
        assert!(codec.decode(&mut buf).unwrap().is_none());
        assert!(buf.is_empty(), "codec must consume what it buffered");
        assert_eq!(codec.buffer, "{\"a\": ");
    }

    #[test]
    fn test_decode_emits_only_complete_lines() {
        let mut codec = PrettyCodec::json("base16-ocean.dark");
        let mut buf = BytesMut::from(&b"line one\nline two"[..]);

        let out = codec.decode(&mut buf).unwrap().expect("complete line");
        assert_eq!(strip_ansi(&out), "line one\n");
        // The trailing partial line is held back for the next call.
        assert_eq!(codec.buffer, "line two");
    }

    #[test]
    fn test_decode_joins_buffered_prefix_with_next_chunk() {
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");

        let mut buf = BytesMut::from(&b"hello "[..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        let mut buf = BytesMut::from(&b"world\n"[..]);
        let out = codec.decode(&mut buf).unwrap().expect("complete line");
        assert_eq!(strip_ansi(&out), "hello world\n");
    }

    #[test]
    fn test_decode_eof_flushes_trailing_partial_line() {
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");

        let mut buf = BytesMut::from(&b"no trailing newline"[..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        let mut empty = BytesMut::new();
        let out = codec.decode_eof(&mut empty).unwrap().expect("flushed tail");
        assert_eq!(strip_ansi(&out), "no trailing newline\n");
    }

    #[test]
    fn test_decode_eof_consumes_remaining_bytes_in_src() {
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");
        let mut buf = BytesMut::from(&b"tail bytes"[..]);

        let out = codec.decode_eof(&mut buf).unwrap().expect("flushed tail");
        assert_eq!(strip_ansi(&out), "tail bytes\n");
        assert!(buf.is_empty());
    }

    #[test]
    fn test_decode_eof_is_idempotent() {
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");
        let mut buf = BytesMut::from(&b"x"[..]);

        assert!(codec.decode_eof(&mut buf).unwrap().is_some());
        // Second call must not re-emit the same text.
        let mut empty = BytesMut::new();
        assert!(codec.decode_eof(&mut empty).unwrap().is_none());
    }

    #[test]
    fn test_decode_eof_with_nothing_buffered_yields_nothing() {
        let mut codec = PrettyCodec::json("base16-ocean.dark");
        let mut empty = BytesMut::new();
        assert!(codec.decode_eof(&mut empty).unwrap().is_none());
    }

    #[test]
    fn test_streaming_a_document_in_chunks_reassembles_it() {
        let doc = "{\n  \"a\": 1,\n  \"b\": [2, 3]\n}\n";
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");
        let mut seen = String::new();

        // Feed the document a few bytes at a time.
        for chunk in doc.as_bytes().chunks(5) {
            let mut buf = BytesMut::from(chunk);
            if let Some(out) = codec.decode(&mut buf).unwrap() {
                seen.push_str(&strip_ansi(&out));
            }
        }
        let mut empty = BytesMut::new();
        if let Some(out) = codec.decode_eof(&mut empty).unwrap() {
            seen.push_str(&strip_ansi(&out));
        }

        assert_eq!(seen, doc);
    }

    #[test]
    fn test_invalid_utf8_is_replaced_not_fatal() {
        let mut codec = PrettyCodec::new("NoSuchSyntax", "base16-ocean.dark");
        let mut buf = BytesMut::from(&[0xff, 0xfe, b'\n'][..]);

        let out = codec.decode(&mut buf).unwrap().expect("lossy line");
        assert!(
            strip_ansi(&out).contains('\u{fffd}'),
            "expected replacement chars"
        );
    }
}
