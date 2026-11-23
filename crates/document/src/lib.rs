//! Structured manuscript documents.
//!
//! Content is stored as identified blocks, never as one giant text field.
//! The schema version lives on the document so the engine can migrate later
//! without tying storage to a UI representation.

mod diff;
mod html;
pub use html::escape_html;
mod model;
mod split;
mod track;

pub use diff::{
    compare_documents, compare_text, DiffOp, DiffParagraph, DiffSpan, DocumentDiff, ParagraphChange,
};
pub use model::{
    normalize_link, Block, BlockKind, Document, Highlight, Inline, InlineNote, ListItem, Marks, NavItem, NavKind,
    NoteKind, ReviewItem, ReviewKind, ReviewTarget,
};

pub use split::{shown_length_of, split_baseline, split_blocks, SplitPoint};
pub use track::{accept_change, reject_change, tracked_changes, TrackedChange};

pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;
