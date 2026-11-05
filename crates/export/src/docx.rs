use docx_rs::{Docx, DocxError, Paragraph, Run, RunFonts};
use writing_studio_document::{BlockKind, Document, Inline};

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
        paragraph = paragraph.add_run(run);
    }
    paragraph
}
