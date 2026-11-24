//! Project format 3: the parts of a project that are plain text.
//!
//! A scene, and every other item with text, is stored as a Markdown file:
//! its details as front matter, then its text. It is what the page edits, and
//! any Markdown reader opens it.
//!
//! The format is described in `documentation/PROJECT_FORMAT.md`.

mod canonical;
mod header;
mod links;
mod read;
mod scene;
#[cfg(test)]
mod tests;
mod write;

pub use canonical::{canonical_blocks, same_content};
pub use header::{split_header, Header, HeaderError, Value};
pub use links::{rename_wikilinks, same_title, wikilinks, WikiLink};
pub use read::markdown_to_blocks;
pub use scene::{file_name, slug, SceneFile, SceneHeader};
pub use write::{blocks_to_markdown, write_markdown, NoteStyle};
