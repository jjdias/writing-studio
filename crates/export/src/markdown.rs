use writing_studio_document::{Block, BlockKind, Document, Inline};

use crate::compile::{collect_sections, CompileInput};

pub fn compile_markdown(input: &CompileInput<'_>) -> String {
    let mut out = Vec::new();
    for section in collect_sections(input) {
        let hashes = "#".repeat(section.heading_level as usize);
        out.push(format!("{hashes} {}", section.title));
        if let Some(document) = section.document {
            let body = document_markdown(document);
            if !body.trim().is_empty() {
                out.push(body);
            }
        }
    }
    out.join("\n\n") + "\n"
}

pub fn document_markdown(document: &Document) -> String {
    document
        .blocks
        .iter()
        .map(block_markdown)
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn block_markdown(block: &Block) -> String {
    match &block.kind {
        BlockKind::Paragraph { inlines } => inlines_markdown(inlines),
        BlockKind::Heading { level, inlines } => {
            format!(
                "{} {}",
                "#".repeat((*level).clamp(1, 6) as usize),
                inlines_markdown(inlines)
            )
        }
        BlockKind::Quote { inlines } => format!("> {}", inlines_markdown(inlines)),
        BlockKind::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let marker = if *ordered {
                    format!("{}.", index + 1)
                } else {
                    "-".into()
                };
                format!("{marker} {}", inlines_markdown(&item.inlines))
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn inlines_markdown(inlines: &[Inline]) -> String {
    inlines.iter().map(inline_markdown).collect()
}

fn inline_markdown(inline: &Inline) -> String {
    let mut text = escape_markdown(&inline.text);
    if inline.marks.strikethrough {
        text = format!("~~{text}~~");
    }
    if inline.marks.underline {
        text = format!("<u>{text}</u>");
    }
    if inline.marks.italic {
        text = format!("*{text}*");
    }
    if inline.marks.bold {
        text = format!("**{text}**");
    }
    if let Some(url) = &inline.marks.link {
        text = format!("[{text}]({url})");
    }
    text
}

fn escape_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
}
