use docx_rs::{
    Docx, DocxError, Hyperlink, HyperlinkType, LineSpacing, PageMargin, Paragraph, Run, RunFonts, SpecialIndentType,
};
use writing_studio_document::{normalize_link, BlockKind, Document, Inline};
use writing_studio_project::{NodeKind, PageSetup};

use crate::compile::{collect_sections, CompileInput};

/// A tenth of an inch, in twentieths of a point.
const TENTH_INCH: i32 = 144;
/// The first-line indent of a paragraph: half an inch.
const INDENT: i32 = 720;

/// The profile's page as Word applies it to each paragraph.
struct Style {
    font: String,
    /// Body text, in half points.
    size: usize,
    line: i32,
    indent: bool,
}

impl Style {
    fn of(page: &PageSetup) -> Self {
        Self {
            font: page.font.clone(),
            size: page.font_size as usize * 2,
            line: page.line_spacing.word_line(),
            indent: page.indent,
        }
    }

    fn fonts(&self) -> RunFonts {
        RunFonts::new().ascii(&self.font).hi_ansi(&self.font)
    }

    /// A body paragraph: indented, or with a space after it.
    fn body(&self, paragraph: Paragraph) -> Paragraph {
        if self.indent {
            paragraph
                .line_spacing(LineSpacing::new().line(self.line))
                .indent(None, Some(SpecialIndentType::FirstLine(INDENT)), None, None)
        } else {
            let after = (self.size * 10) as u32;
            paragraph.line_spacing(LineSpacing::new().line(self.line).after(after))
        }
    }

    /// A heading's size: larger for the book and its parts and chapters.
    fn heading_size(&self, level: u8) -> usize {
        match level {
            1 => self.size * 2,
            2 => self.size * 3 / 2,
            3 => self.size * 5 / 4,
            _ => self.size,
        }
    }
}

pub fn compile_docx(input: &CompileInput<'_>) -> Result<Vec<u8>, DocxError> {
    let style = Style::of(input.page);
    let (width, height) = input.page.size.twips();
    let margin = input.page.margin as i32 * TENTH_INCH;
    let mut docx = Docx::new()
        .page_size(width, height)
        .page_margin(PageMargin::new().top(margin).bottom(margin).left(margin).right(margin))
        .default_fonts(style.fonts())
        .default_size(style.size);
    let mut first = true;
    for section in collect_sections(input) {
        // Each part and chapter starts a page, and so does what follows the
        // title page; the first section starts the first page anyway.
        let new_page = !first
            && input.page.chapter_new_page
            && (section.kind.is_some_and(|kind| matches!(kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter))
                || section.after_title_page);
        docx = docx.add_paragraph(heading_paragraph(&style, &section.title, section.heading_level).page_break_before(new_page));
        if let Some(document) = section.document {
            docx = add_document(docx, &style, document);
        }
        first = false;
    }
    let mut buf = Vec::new();
    docx.build().pack(&mut std::io::Cursor::new(&mut buf))?;
    Ok(buf)
}

fn add_document(mut docx: Docx, style: &Style, document: &Document) -> Docx {
    for block in &document.blocks {
        match &block.kind {
            BlockKind::Paragraph { inlines } => {
                docx = docx.add_paragraph(style.body(add_inlines(Paragraph::new(), style, inlines, style.size, false, false)));
            }
            BlockKind::Heading { level, inlines } => {
                let size = style.heading_size(level.saturating_add(1));
                docx = docx.add_paragraph(add_inlines(Paragraph::new(), style, inlines, size, true, false));
            }
            BlockKind::Quote { inlines } => {
                let paragraph = add_inlines(Paragraph::new(), style, inlines, style.size, false, true);
                docx = docx.add_paragraph(paragraph.indent(Some(INDENT), None, Some(INDENT), None));
            }
            BlockKind::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered { format!("{}. ", index + 1) } else { "• ".into() };
                    let mut paragraph = Paragraph::new().add_run(Run::new().add_text(&marker).size(style.size).fonts(style.fonts()));
                    paragraph = add_inlines(paragraph, style, &item.inlines, style.size, false, false);
                    docx = docx.add_paragraph(paragraph.line_spacing(LineSpacing::new().line(style.line)));
                }
            }
            BlockKind::Code { .. } => {
                let text = block.plain_text();
                let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').collect() };
                for line in lines {
                    docx = docx.add_paragraph(Paragraph::new().add_run(
                        Run::new().add_text(line).size(style.size.saturating_sub(2)).fonts(RunFonts::new().ascii("Consolas")),
                    ));
                }
            }
            BlockKind::SceneBreak => {
                docx = docx.add_paragraph(
                    Paragraph::new()
                        .align(docx_rs::AlignmentType::Center)
                        .line_spacing(LineSpacing::new().line(style.line))
                        .add_run(Run::new().add_text("* * *").size(style.size).fonts(style.fonts())),
                );
            }
        }
    }
    docx
}

fn heading_paragraph(style: &Style, title: &str, level: u8) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(title).bold().size(style.heading_size(level)).fonts(style.fonts()))
}

fn add_inlines(
    mut paragraph: Paragraph,
    style: &Style,
    inlines: &[Inline],
    size: usize,
    force_bold: bool,
    force_italic: bool,
) -> Paragraph {
    if inlines.is_empty() {
        return paragraph.add_run(Run::new().add_text(""));
    }
    for inline in inlines {
        let run = styled_run(style, inline, size, force_bold, force_italic);
        if let Some(url) = inline.marks.link.as_deref().and_then(normalize_link) {
            paragraph = paragraph.add_hyperlink(
                Hyperlink::new(url, HyperlinkType::External).add_run(run.color("1D4E89").underline("single")),
            );
        } else {
            paragraph = paragraph.add_run(run);
        }
        if let Some(note) = inline.marks.compile_note() {
            paragraph = paragraph.add_run(
                Run::new()
                    .add_text(format!(" [{note}]"))
                    .italic()
                    .size(size.saturating_sub(4).max(16))
                    .color("6B5E48")
                    .fonts(style.fonts()),
            );
        }
    }
    paragraph
}

fn styled_run(style: &Style, inline: &Inline, size: usize, force_bold: bool, force_italic: bool) -> Run {
    let mut run = Run::new().add_text(&inline.text).size(size).fonts(style.fonts());
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
