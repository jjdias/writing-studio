use writing_studio_document::Block;
use writing_studio_format::{write_markdown, NoteStyle};

use crate::compile::{collect_sections, CompileInput};

/// The book as one Markdown file: a heading for each section, then its
/// text. Open notes are footnotes, numbered across the whole book.
pub fn compile_markdown(input: &CompileInput<'_>) -> String {
    let mut blocks = Vec::new();
    for section in collect_sections(input) {
        blocks.push(Block::heading(section.heading_level, section.title));
        if let Some(document) = section.document {
            blocks.extend(document.blocks.iter().cloned());
        }
    }
    write_markdown(&blocks, NoteStyle::Footnotes)
}
