use writing_studio_document::{normalize_link, Block, BlockKind, Document, Inline};

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
    let mut notes = Vec::new();
    let mut blocks: Vec<String> = document
        .blocks
        .iter()
        .map(|block| block_markdown(block, &mut notes))
        .filter(|block| !block.is_empty())
        .collect();
    for (index, note) in notes.iter().enumerate() {
        blocks.push(format!("[^{}]: {note}", index + 1));
    }
    blocks.join("\n\n")
}

fn block_markdown(block: &Block, notes: &mut Vec<String>) -> String {
    match &block.kind {
        BlockKind::Paragraph { inlines } => inlines_markdown(inlines, notes),
        BlockKind::Heading { level, inlines } => {
            format!(
                "{} {}",
                "#".repeat((*level).clamp(1, 6) as usize),
                inlines_markdown(inlines, notes)
            )
        }
        BlockKind::Quote { inlines } => format!("> {}", inlines_markdown(inlines, notes)),
        BlockKind::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let marker = if *ordered {
                    format!("{}.", index + 1)
                } else {
                    "-".into()
                };
                format!("{marker} {}", inlines_markdown(&item.inlines, notes))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        BlockKind::Code { .. } => fence_code(&block.plain_text()),
        BlockKind::SceneBreak => "* * *".into(),
    }
}

fn fence_code(text: &str) -> String {
    let fence = if text.contains("```") { "~~~~" } else { "```" };
    format!("{fence}\n{text}\n{fence}")
}

fn inlines_markdown(inlines: &[Inline], notes: &mut Vec<String>) -> String {
    inlines
        .iter()
        .map(|inline| inline_markdown(inline, notes))
        .collect()
}

fn inline_markdown(inline: &Inline, notes: &mut Vec<String>) -> String {
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
    if let Some(url) = inline.marks.link.as_deref().and_then(normalize_link) {
        text = format!("[{text}]({url})");
    }
    if let Some(note) = inline
        .marks
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
    {
        let n = notes.len() + 1;
        notes.push(note.replace('\n', " "));
        text.push_str(&format!("[^{n}]"));
    }
    text
}

fn escape_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
}
