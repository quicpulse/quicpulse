//! Output writer for HTTP messages

use std::io::{self, Write};

use crate::output::formatters::{ColorFormatter, ColorStyle, JsonFormatterOptions, format_json};

/// Output options controlling what to display
#[derive(Debug, Clone)]
pub struct OutputOptions {
    /// Show request headers
    pub request_headers: bool,
    /// Show request body
    pub request_body: bool,
    /// Show response headers
    pub response_headers: bool,
    /// Show response body
    pub response_body: bool,
    /// Show metadata (timing, etc.)
    pub metadata: bool,
}

impl OutputOptions {
    /// Default: show response headers and body
    pub fn default_output() -> Self {
        Self {
            request_headers: false,
            request_body: false,
            response_headers: true,
            response_body: true,
            metadata: false,
        }
    }

    /// Verbose: show request and response
    pub fn verbose() -> Self {
        Self {
            request_headers: true,
            request_body: true,
            response_headers: true,
            response_body: true,
            metadata: false,
        }
    }

    /// Headers only
    pub fn headers_only() -> Self {
        Self {
            request_headers: false,
            request_body: false,
            response_headers: true,
            response_body: false,
            metadata: false,
        }
    }

    /// Body only
    pub fn body_only() -> Self {
        Self {
            request_headers: false,
            request_body: false,
            response_headers: false,
            response_body: true,
            metadata: false,
        }
    }

    /// Parse from string like "hb" (headers + body)
    pub fn from_str(s: &str) -> Self {
        let mut opts = Self {
            request_headers: false,
            request_body: false,
            response_headers: false,
            response_body: false,
            metadata: false,
        };

        for c in s.chars() {
            match c {
                'H' => opts.request_headers = true,
                'B' => opts.request_body = true,
                'h' => opts.response_headers = true,
                'b' => opts.response_body = true,
                'm' => opts.metadata = true,
                _ => {}
            }
        }

        opts
    }

    /// Check if any output is enabled
    pub fn any(&self) -> bool {
        self.request_headers || self.request_body || 
        self.response_headers || self.response_body || 
        self.metadata
    }
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self::default_output()
    }
}

/// Processing options for output formatting
#[derive(Debug, Clone)]
pub struct ProcessingOptions {
    /// Color style for syntax highlighting
    pub style: ColorStyle,
    /// JSON formatting options
    pub json: JsonFormatterOptions,
    /// Whether colors are enabled
    pub colors: bool,
    /// Pretty print (format JSON/XML)
    pub pretty: PrettyOption,
}

// Note: PrettyOption is defined in output::options and re-exported from super
pub use super::options::PrettyOption;

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            style: ColorStyle::Auto,
            json: JsonFormatterOptions::default(),
            colors: true,
            pretty: PrettyOption::All,
        }
    }
}

/// Write HTTP response to output
pub fn write_response<W: Write>(
    writer: &mut W,
    headers: &str,
    body: &str,
    content_type: Option<&str>,
    output_opts: &OutputOptions,
    proc_opts: &ProcessingOptions,
) -> io::Result<()> {
    // Write headers
    if output_opts.response_headers {
        let formatted_headers = if proc_opts.colors && proc_opts.pretty != PrettyOption::None {
            let formatter = ColorFormatter::new(proc_opts.style.clone());
            formatter.format_headers(headers)
        } else {
            headers.to_string()
        };
        
        writer.write_all(formatted_headers.as_bytes())?;
        
        if output_opts.response_body && !body.is_empty() {
            writer.write_all(b"\n")?;
        }
    }

    // Write body
    if output_opts.response_body && !body.is_empty() {
        let formatted_body = format_body(body, content_type, proc_opts);
        writer.write_all(formatted_body.as_bytes())?;
    }

    writer.flush()
}

/// Format the response body based on content type
fn format_body(body: &str, content_type: Option<&str>, opts: &ProcessingOptions) -> String {
    let mime = content_type.unwrap_or("text/plain");
    let base_mime = mime.split(';').next().unwrap_or(mime).trim();

    // Format JSON
    if base_mime == "application/json" || base_mime.ends_with("+json") {
        if matches!(opts.pretty, PrettyOption::All | PrettyOption::Format) {
            if let Ok(formatted) = format_json(body, &opts.json) {
                let result = if opts.colors && matches!(opts.pretty, PrettyOption::All | PrettyOption::Colors) {
                    let formatter = ColorFormatter::new(opts.style.clone());
                    formatter.format_json(&formatted)
                } else {
                    formatted
                };
                return result;
            }
        }
    }

    // Apply syntax highlighting for other types
    if opts.colors && matches!(opts.pretty, PrettyOption::All | PrettyOption::Colors) {
        let formatter = ColorFormatter::new(opts.style.clone());
        formatter.format_by_mime(body, base_mime)
    } else {
        body.to_string()
    }
}
