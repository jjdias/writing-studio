use writing_studio_core::Id;
use writing_studio_document::{Block, BlockKind, Inline, ListItem, Marks};
use writing_studio_format::{write_markdown, NoteStyle};
use writing_studio_project::SceneSeparator;

use crate::compile::{collect_sections, CompileInput, MatterPage};

/// The book as one Markdown file: a heading for each section that has one,
/// a scene break where the profile sets one, then the text. Open notes are
/// footnotes, numbered across the whole book. An empty line between scenes
/// is what Markdown puts between paragraphs anyway, so it adds nothing.
/// Markdown has no pages, so matter is set one section after another.
pub fn compile_markdown(input: &CompileInput<'_>) -> String {
    let mut blocks = Vec::new();
    for section in collect_sections(input) {
        if section.break_before == Some(SceneSeparator::Break) {
            blocks.push(Block { id: Id::new(), kind: BlockKind::SceneBreak });
        }
        if let Some(title) = section.title {
            blocks.push(Block::heading(section.heading_level, title));
        }
        match section.matter {
            Some(MatterPage::Title { subtitle, author }) => {
                if !subtitle.is_empty() {
                    blocks.push(italic(subtitle));
                }
                if !author.is_empty() {
                    blocks.push(Block::paragraph(author));
                }
            }
            Some(MatterPage::Copyright(line)) => blocks.push(Block::paragraph(line)),
            Some(MatterPage::Dedication(lines)) => blocks.extend(lines.into_iter().map(italic)),
            Some(MatterPage::Contents(entries)) => {
                // Markdown lists here are flat, so a chapter under a part is
                // set off by a dash instead of an indent.
                let items = entries.into_iter().map(|entry| {
                    let lead = "– ".repeat(entry.depth as usize);
                    format!("{lead}{}", entry.title)
                });
                blocks.push(list(items));
            }
            Some(MatterPage::AboutAuthor(paragraphs)) => blocks.extend(paragraphs.into_iter().map(Block::paragraph)),
            Some(MatterPage::AlsoBy(titles)) => blocks.push(list(titles.into_iter())),
            None => {}
        }
        if let Some(document) = section.document {
            blocks.extend(document.blocks.iter().cloned());
        }
    }
    write_markdown(&blocks, NoteStyle::Footnotes)
}

fn italic(text: String) -> Block {
    let inline = Inline { id: Id::new(), text, marks: Marks { italic: true, ..Marks::default() } };
    Block { id: Id::new(), kind: BlockKind::Paragraph { inlines: vec![inline] } }
}

fn list(items: impl Iterator<Item = String>) -> Block {
    let items = items.map(|text| ListItem { id: Id::new(), inlines: vec![Inline::text(text)] }).collect();
    Block { id: Id::new(), kind: BlockKind::List { ordered: false, items } }
}
