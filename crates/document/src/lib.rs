//! Structured manuscript documents.
//!
//! Content is stored as identified blocks, never as one giant text field.
//! The schema version lives on the document so the engine can migrate later
//! without tying storage to a UI representation.

mod diff;
mod html;
mod model;

pub use diff::{
    compare_documents, compare_text, DiffOp, DiffParagraph, DiffSpan, DocumentDiff, ParagraphChange,
};
pub use model::{
    normalize_link, Block, BlockKind, Document, Highlight, Inline, InlineNote, ListItem, Marks,
    NoteKind, ReviewItem, ReviewKind, ReviewTarget,
};

pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;
