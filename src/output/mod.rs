//! Output handling (formatting, streams, writing)

pub mod codec;
pub mod error;
pub mod formatters;
pub mod lexers;
pub mod models;
pub mod options;
pub mod pager;
pub mod streams;
pub mod terminal;
pub mod writer;

pub use codec::{EncodedCodec, PrettyCodec, RawCodec};
pub use error::{StreamError, StreamResult};
pub use options::{OutputFlags, PrettyOption};
pub use pager::{PagerConfig, PagerWriter, get_pager_command, should_page, write_with_pager};
pub use terminal::{colors, fg, bold_fg, colorize, bold, success, error, warning, info, label, muted, RESET};

