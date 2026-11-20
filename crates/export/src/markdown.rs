use writing_studio_document::{Block, BlockKind};
use writing_studio_format::{write_markdown, NoteStyle};
use writing_studio_project::SceneSeparator;

use crate::compile::{collect_sections, CompileInput};

/// The book as one Markdown file: a heading for each section that has one,
/// a scene break where the profile sets one, then the text. Open notes are
/// footnotes, numbered across the whole book. An empty line between scenes
/// is what Markdown puts between paragraphs anyway, so it adds nothing.
pub fn compile_markdown(input: &CompileInput<'_>) -> String {
    let mut blocks = Vec::new();
    for section in collect_sections(input) {
        if section.break_before == Some(SceneSeparator::Break) {
            blocks.push(Block { id: writing_studio_core::Id::new(), kind: BlockKind::SceneBreak });
        }
        if let Some(title) = section.title {
            blocks.push(Block::heading(section.heading_level, title));
        }
        if let Some(document) = section.document {
            blocks.extend(document.blocks.iter().cloned());
        }
    }
    write_markdown(&blocks, NoteStyle::Footnotes)
}
