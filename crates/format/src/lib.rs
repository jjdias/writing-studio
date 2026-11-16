//! Project format 2: the parts of a project that are plain text.
//!
//! A scene is stored as an HTML file, which holds the document exactly and
//! opens in any browser. The same scene can be written as Markdown, for
//! taking a book to another program, and read from Markdown.
//!
//! The format is described in `documentation/PROJECT_FORMAT_2.md`.

mod canonical;
mod header;
mod html;
mod read;
mod scene;
#[cfg(test)]
mod tests;
mod write;

pub use canonical::{canonical_blocks, same_content};
pub use header::{split_header, Header, HeaderError, Value};
pub use html::{blocks_to_html, html_to_blocks, stored_blocks};
pub use read::markdown_to_blocks;
pub use scene::{file_name, slug, SceneFile, SceneHeader, STYLESHEET};
pub use write::{blocks_to_markdown, write_markdown, NoteStyle};
