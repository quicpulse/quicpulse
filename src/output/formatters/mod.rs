//! Output formatters

pub mod colors;
pub mod headers;
pub mod json;
pub mod xml;

pub use colors::{ColorFormatter, ColorStyle};
pub use headers::format_headers;
pub use json::{format_json, JsonFormatterOptions};
pub use xml::format_xml;
