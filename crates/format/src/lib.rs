//! Project format 2: the parts of a project that are plain text.
//!
//! A scene is a Markdown file. This crate writes a scene's document as
//! Markdown and reads it back to the same document, so that a project can be
//! read, edited, and kept under version control without this program.
//!
//! The format is described in `documentation/PROJECT_FORMAT_2.md`.

mod canonical;
mod header;
mod read;
mod scene;
#[cfg(test)]
mod tests;
mod write;

pub use canonical::{canonical_blocks, same_content};
pub use header::{split_header, Header, HeaderError, Value};
pub use read::markdown_to_blocks;
pub use scene::{file_name, SceneFile, SceneHeader};
pub use write::blocks_to_markdown;
