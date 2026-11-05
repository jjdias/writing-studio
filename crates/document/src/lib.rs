//! Structured manuscript documents.
//!
//! Content is stored as identified blocks, never as one giant text field.
//! The schema version lives on the document so the engine can migrate later
//! without tying storage to a UI representation.

mod html;
mod model;

pub use model::{Block, BlockKind, Document, Inline, ListItem, Marks};

pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;
