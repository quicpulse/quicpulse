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
pub use pager::{get_pager_command, should_page, write_with_pager, PagerConfig, PagerWriter};
pub use terminal::{
    bold, bold_fg, colorize, colors, error, fg, info, label, muted, success, warning, RESET,
};
