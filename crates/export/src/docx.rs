use docx_rs::{Docx, DocxError, Hyperlink, HyperlinkType, Paragraph, Run, RunFonts};
use writing_studio_document::{normalize_link, BlockKind, Document, Inline};

use crate::compile::{collect_sections, CompileInput};

pub fn compile_docx(input: &CompileInput<'_>) -> Result<Vec<u8>, DocxError> {
    let mut docx = Docx::new();
    for section in collect_sections(input) {
        docx = docx.add_paragraph(heading_paragraph(&section.title, section.heading_level));
        if let Some(document) = section.document {
            docx = add_document(docx, document);
        }
    }
    let mut buf = Vec::new();
    docx.build().pack(&mut std::io::Cursor::new(&mut buf))?;
    Ok(buf)
}

fn add_document(mut docx: Docx, document: &Document) -> Docx {
    for block in &document.blocks {
        match &block.kind {
            BlockKind::Paragraph { inlines } => {
                docx = docx.add_paragraph(inlines_paragraph(inlines, 22));
            }
            BlockKind::Heading { level, inlines } => {
                let size = match *level {
                    1 => 36,
                    2 => 32,
                    3 => 28,
                    _ => 24,
                };
                docx = docx.add_paragraph(inlines_paragraph_marked(inlines, size, true, false));
            }
            BlockKind::Quote { inlines } => {
                docx = docx.add_paragraph(inlines_paragraph_marked(inlines, 22, false, true));
            }
            BlockKind::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered {
                        format!("{}. ", index + 1)
                    } else {
                        "• ".into()
                    };
                    let mut paragraph = Paragraph::new().add_run(Run::new().add_text(&marker));
                    paragraph = add_inlines(paragraph, &item.inlines, 22, false, false);
                    docx = docx.add_paragraph(paragraph);
                }
            }
            BlockKind::Code { .. } => {
                let text = block.plain_text();
                let lines: Vec<&str> = if text.is_empty() {
                    vec![""]
                } else {
                    text.split('\n').collect()
                };
                for line in lines {
                    docx = docx.add_paragraph(Paragraph::new().add_run(
                        Run::new()
                            .add_text(line)
                            .size(20)
                            .fonts(RunFonts::new().ascii("Consolas")),
                    ));
                }
            }
            BlockKind::SceneBreak => {
                docx = docx.add_paragraph(Paragraph::new().add_run(
                    Run::new()
                        .add_text("* * *")
                        .italic()
                        .size(22)
                        .fonts(RunFonts::new().ascii("Times New Roman")),
                ));
            }
        }
    }
    docx
}

fn heading_paragraph(title: &str, level: u8) -> Paragraph {
    let size = match level {
        1 => 48,
        2 => 36,
        3 => 28,
        _ => 24,
    };
    Paragraph::new().add_run(
        Run::new()
            .add_text(title)
            .bold()
            .size(size)
            .fonts(RunFonts::new().ascii("Times New Roman")),
    )
}

fn inlines_paragraph(inlines: &[Inline], size: usize) -> Paragraph {
    inlines_paragraph_marked(inlines, size, false, false)
}

fn inlines_paragraph_marked(inlines: &[Inline], size: usize, bold: bool, italic: bool) -> Paragraph {
    add_inlines(Paragraph::new(), inlines, size, bold, italic)
}

fn add_inlines(
    mut paragraph: Paragraph,
    inlines: &[Inline],
    size: usize,
    force_bold: bool,
    force_italic: bool,
) -> Paragraph {
    if inlines.is_empty() {
        return paragraph.add_run(Run::new().add_text(""));
    }
    for inline in inlines {
        let run = styled_run(inline, size, force_bold, force_italic);
        if let Some(url) = inline.marks.link.as_deref().and_then(normalize_link) {
            paragraph = paragraph.add_hyperlink(
                Hyperlink::new(url, HyperlinkType::External)
                    .add_run(run.color("1D4E89").underline("single")),
            );
        } else {
            paragraph = paragraph.add_run(run);
        }
        if let Some(note) = inline
            .marks
            .comment
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
        {
            paragraph = paragraph.add_run(
                Run::new()
                    .add_text(format!(" [{note}]"))
                    .italic()
                    .size(size.saturating_sub(4).max(16))
                    .color("6B5E48")
                    .fonts(RunFonts::new().ascii("Times New Roman")),
            );
        }
    }
    paragraph
}

fn styled_run(inline: &Inline, size: usize, force_bold: bool, force_italic: bool) -> Run {
    let mut run = Run::new()
        .add_text(&inline.text)
        .size(size)
        .fonts(RunFonts::new().ascii("Times New Roman"));
    if force_bold || inline.marks.bold {
        run = run.bold();
    }
    if force_italic || inline.marks.italic {
        run = run.italic();
    }
    if inline.marks.strikethrough {
        run = run.strike();
    }
    if inline.marks.underline {
        run = run.underline("single");
    }
    run
}
