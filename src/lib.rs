//! diff-pretty renders Git-style unified patches with fixed colors, line
//! numbers, word emphasis, and an optional built-in pager. It reads patches
//! from stdin and does not read user configuration or invoke Git.

pub mod align;
pub mod config;
pub mod edits;
pub mod render;
pub mod source;

pub use render::{
    RenderSession, render, render_document, render_reader_document, render_reader_to, render_to,
};

/// Fixed terminal width used when stdout is not a terminal.
pub const WIDTH: usize = 80;
