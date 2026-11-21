use docx_rs::{
    AlignmentType, Docx, DocxError, Hyperlink, HyperlinkType, LineSpacing, PageMargin, Paragraph, Run, RunFonts, SpecialIndentType,
};
use writing_studio_document::{normalize_link, BlockKind, Document, Inline};
use writing_studio_project::{NodeKind, PageSetup, SceneSeparator};

use crate::compile::{collect_sections, CompileInput, MatterPage};

/// A tenth of an inch, in twentieths of a point.
const TENTH_INCH: i32 = 144;
/// The first-line indent of a paragraph: half an inch.
const INDENT: i32 = 720;
/// How far down the page a title or a dedication starts: two inches.
const TITLE_DROP: u32 = 2880;

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
    let page = &input.profile.page;
    let style = Style::of(page);
    let (width, height) = page.size.twips();
    let margin = page.margin as i32 * TENTH_INCH;
    let mut docx = Docx::new()
        .page_size(width, height)
        .page_margin(PageMargin::new().top(margin).bottom(margin).left(margin).right(margin))
        .default_fonts(style.fonts())
        .default_size(style.size);
    let mut first = true;
    for section in collect_sections(input) {
        // Each page of matter starts a page, and so does the text after the
        // front matter. Each part and chapter does when the profile says so.
        // The first section starts the first page anyway.
        let new_page = !first
            && (section.matter.is_some()
                || section.after_front_matter
                || (page.chapter_new_page
                    && section.kind.is_some_and(|kind| matches!(kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter))));
        let mut paragraphs = Vec::new();
        match section.break_before {
            Some(SceneSeparator::Break) => paragraphs.push(scene_break(&style)),
            Some(SceneSeparator::Blank) => paragraphs.push(Paragraph::new().add_run(Run::new().add_text("").size(style.size))),
            _ => {}
        }
        match (&section.matter, &section.title) {
            (Some(MatterPage::Title { subtitle, author }), Some(title)) => {
                paragraphs.extend(title_page(&style, title, subtitle, author));
            }
            (_, Some(title)) => paragraphs.push(heading_paragraph(&style, title, section.heading_level)),
            _ => {}
        }
        if let Some(matter) = &section.matter {
            paragraphs.extend(matter_paragraphs(&style, matter));
        }
        if let Some(document) = section.document {
            paragraphs.extend(document_paragraphs(&style, document));
        }
        for (index, paragraph) in paragraphs.into_iter().enumerate() {
            docx = docx.add_paragraph(if index == 0 && new_page { paragraph.page_break_before(true) } else { paragraph });
        }
        first = false;
    }
    let mut buf = Vec::new();
    docx.build().pack(&mut std::io::Cursor::new(&mut buf))?;
    Ok(buf)
}

/// The title a third of the way down the page, then the subtitle and the
/// author, all centered.
fn title_page(style: &Style, title: &str, subtitle: &str, author: &str) -> Vec<Paragraph> {
    let centered = |text: &str, size: usize| {
        Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().line(style.line))
            .add_run(Run::new().add_text(text).size(size).fonts(style.fonts()))
    };
    let mut paragraphs = vec![Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(LineSpacing::new().before(TITLE_DROP).after(240))
        .add_run(Run::new().add_text(title).bold().size(style.heading_size(1)).fonts(style.fonts()))];
    if !subtitle.is_empty() {
        paragraphs.push(centered(subtitle, style.heading_size(3)));
    }
    if !author.is_empty() {
        paragraphs.push(centered(author, style.size));
    }
    paragraphs
}

fn matter_paragraphs(style: &Style, matter: &MatterPage) -> Vec<Paragraph> {
    let line = |text: &str, italic: bool, align: AlignmentType| {
        let mut run = Run::new().add_text(text).size(style.size).fonts(style.fonts());
        if italic {
            run = run.italic();
        }
        Paragraph::new().align(align).line_spacing(LineSpacing::new().line(style.line)).add_run(run)
    };
    match matter {
        MatterPage::Title { .. } => Vec::new(),
        MatterPage::Copyright(text) => vec![line(text, false, AlignmentType::Left)],
        MatterPage::Dedication(lines) => lines
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let paragraph = line(text, true, AlignmentType::Center);
                // The dedication starts down the page, as the title does.
                if index == 0 {
                    paragraph.line_spacing(LineSpacing::new().before(TITLE_DROP).line(style.line))
                } else {
                    paragraph
                }
            })
            .collect(),
        MatterPage::Contents(entries) => entries
            .iter()
            .map(|entry| {
                line(&entry.title, false, AlignmentType::Left)
                    .indent(Some(INDENT * i32::from(entry.depth)), None, None, None)
            })
            .collect(),
        MatterPage::AboutAuthor(paragraphs) => paragraphs
            .iter()
            .map(|text| style.body(Paragraph::new().add_run(Run::new().add_text(text).size(style.size).fonts(style.fonts()))))
            .collect(),
        MatterPage::AlsoBy(titles) => titles.iter().map(|text| line(text, true, AlignmentType::Left)).collect(),
    }
}

fn document_paragraphs(style: &Style, document: &Document) -> Vec<Paragraph> {
    let mut paragraphs = Vec::new();
    for block in &document.blocks {
        match &block.kind {
            BlockKind::Paragraph { inlines } => {
                paragraphs.push(style.body(add_inlines(Paragraph::new(), style, inlines, style.size, false, false)));
            }
            BlockKind::Heading { level, inlines } => {
                let size = style.heading_size(level.saturating_add(1));
                paragraphs.push(add_inlines(Paragraph::new(), style, inlines, size, true, false));
            }
            BlockKind::Quote { inlines } => {
                let paragraph = add_inlines(Paragraph::new(), style, inlines, style.size, false, true);
                paragraphs.push(paragraph.indent(Some(INDENT), None, Some(INDENT), None));
            }
            BlockKind::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered { format!("{}. ", index + 1) } else { "• ".into() };
                    let mut paragraph = Paragraph::new().add_run(Run::new().add_text(&marker).size(style.size).fonts(style.fonts()));
                    paragraph = add_inlines(paragraph, style, &item.inlines, style.size, false, false);
                    paragraphs.push(paragraph.line_spacing(LineSpacing::new().line(style.line)));
                }
            }
            BlockKind::Code { .. } => {
                let text = block.plain_text();
                let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').collect() };
                for line in lines {
                    paragraphs.push(Paragraph::new().add_run(
                        Run::new().add_text(line).size(style.size.saturating_sub(2)).fonts(RunFonts::new().ascii("Consolas")),
                    ));
                }
            }
            BlockKind::SceneBreak => paragraphs.push(scene_break(style)),
        }
    }
    paragraphs
}

/// A centered "* * *".
fn scene_break(style: &Style) -> Paragraph {
    Paragraph::new()
        .align(docx_rs::AlignmentType::Center)
        .line_spacing(LineSpacing::new().line(style.line))
        .add_run(Run::new().add_text("* * *").size(style.size).fonts(style.fonts()))
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
