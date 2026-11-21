//! The book as one web page: its text, its matter, its notes at the end, and
//! a stylesheet from the profile, all in the one file. Nothing is loaded
//! from anywhere else and nothing runs, so it opens in any browser and can
//! be put on a website as it is.
//!
//! The section writer and the type styles here are shared with the EPUB
//! compile, which writes the same markup as XHTML across several files.

use writing_studio_document::escape_html;
use writing_studio_project::{LineSpacing, NodeKind, PageSetup, SceneSeparator};

use crate::compile::{collect_sections, CompileInput, CompiledSection, MatterPage};

pub fn compile_html(input: &CompileInput<'_>) -> String {
    let profile = input.profile;
    let sections = collect_sections(input);
    let title = book_title(&sections, input.title);
    let same_page = |_: usize| String::new();
    let mut writer = SectionWriter::new(&same_page, "", false);
    let mut body = String::new();
    for section in &sections {
        writer.section(&mut body, section);
    }
    if !writer.notes.is_empty() {
        body.push_str(&writer.notes_section(profile.language.notes(), 2));
    }

    let author = input.author.trim();
    let author_meta =
        if author.is_empty() { String::new() } else { format!("<meta name=\"author\" content=\"{}\">\n", escape_html(author)) };
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"Writing Studio\">\n{author_meta}<title>{title}</title>\n\
         <style>\n{type_css}{web_css}</style>\n</head>\n<body>\n<main>\n{body}</main>\n</body>\n</html>\n",
        lang = profile.language.code(),
        title = escape_html(&title),
        type_css = type_css(&profile.page),
        web_css = WEB_CSS,
    )
}

/// The title page's title, which is the book's own when one book is
/// compiled, or `fallback`.
pub(crate) fn book_title(sections: &[CompiledSection<'_>], fallback: &str) -> String {
    sections
        .iter()
        .find(|section| matches!(section.matter, Some(MatterPage::Title { .. })))
        .and_then(|section| section.title.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// Writes sections one after another, numbering the chapters the contents
/// link to and the notes as they come.
pub(crate) struct SectionWriter<'a> {
    /// Each open note's text and the page its mark is on.
    pub notes: Vec<(String, String)>,
    /// Parts and chapters so far, in the order the contents list them.
    chapters: usize,
    /// The page the `n`th part or chapter (from 1) is on: empty when the
    /// book is one page.
    chapter_page: &'a dyn Fn(usize) -> String,
    /// The page the notes are listed on.
    notes_page: String,
    /// The page being written.
    pub page: String,
    /// XHTML, as an ebook needs: void elements are closed, and a note's
    /// mark says it is one in the ebook's own words.
    xhtml: bool,
}

impl<'a> SectionWriter<'a> {
    pub(crate) fn new(chapter_page: &'a dyn Fn(usize) -> String, notes_page: &str, xhtml: bool) -> Self {
        Self { notes: Vec::new(), chapters: 0, chapter_page, notes_page: notes_page.into(), page: String::new(), xhtml }
    }

    pub(crate) fn section(&mut self, out: &mut String, section: &CompiledSection<'_>) {
        let rule = if self.xhtml { "<hr/>" } else { "<hr>" };
        match section.break_before {
            Some(SceneSeparator::Break) => out.push_str(&format!("{rule}\n")),
            Some(SceneSeparator::Blank) => out.push_str(if self.xhtml { "<hr class=\"blank\"/>\n" } else { "<hr class=\"blank\">\n" }),
            _ => {}
        }
        if let Some(matter) = &section.matter {
            self.matter(out, matter, section.title.as_deref(), section.heading_level);
            return;
        }
        if let Some(heading) = &section.title {
            let level = section.heading_level.clamp(1, 6);
            let opens = matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter | NodeKind::Book));
            let anchor = if matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter)) {
                self.chapters += 1;
                format!(" id=\"chapter-{}\"", self.chapters)
            } else {
                String::new()
            };
            let class = if opens { " class=\"opens\"" } else { "" };
            out.push_str(&format!("<h{level}{anchor}{class}>{}</h{level}>\n", escape_html(heading)));
        }
        if let Some(document) = section.document {
            let (notes, page, notes_page, xhtml) = (&mut self.notes, &self.page, &self.notes_page, self.xhtml);
            let text = document.to_reader_html(&mut |note| {
                notes.push((note.to_string(), page.clone()));
                let n = notes.len();
                let kind = if xhtml { " epub:type=\"noteref\"" } else { "" };
                format!(
                    "<sup class=\"note-ref\"><a id=\"ref-{n}\" href=\"{notes_page}#note-{n}\" role=\"doc-noteref\"{kind}>{n}</a></sup>"
                )
            });
            if self.xhtml {
                out.push_str(&text.replace("<br>", "<br/>").replace("<hr>", "<hr/>"));
            } else {
                out.push_str(&text);
            }
            out.push('\n');
        }
    }

    fn matter(&self, out: &mut String, matter: &MatterPage, title: Option<&str>, level: u8) {
        let level = level.clamp(1, 6);
        let heading = |out: &mut String| {
            if let Some(title) = title {
                out.push_str(&format!("<h{level}>{}</h{level}>\n", escape_html(title)));
            }
        };
        let paragraphs = |out: &mut String, lines: &[String]| {
            for line in lines {
                out.push_str(&format!("<p>{}</p>\n", escape_html(line)));
            }
        };
        match matter {
            MatterPage::Title { subtitle, author } => {
                out.push_str("<header class=\"title-page\">\n");
                if let Some(title) = title {
                    out.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));
                }
                if !subtitle.is_empty() {
                    out.push_str(&format!("<p class=\"subtitle\">{}</p>\n", escape_html(subtitle)));
                }
                if !author.is_empty() {
                    out.push_str(&format!("<p class=\"author\">{}</p>\n", escape_html(author)));
                }
                out.push_str("</header>\n");
            }
            MatterPage::Copyright(line) => {
                out.push_str(&format!("<section class=\"copyright\">\n<p>{}</p>\n</section>\n", escape_html(line)));
            }
            MatterPage::Dedication(lines) => {
                out.push_str("<section class=\"dedication\">\n");
                paragraphs(out, lines);
                out.push_str("</section>\n");
            }
            MatterPage::Contents(entries) => {
                out.push_str("<nav class=\"contents\" role=\"doc-toc\">\n");
                heading(out);
                out.push_str("<ol>\n");
                for (index, entry) in entries.iter().enumerate() {
                    let n = index + 1;
                    out.push_str(&format!(
                        "<li class=\"depth-{}\"><a href=\"{}#chapter-{n}\">{}</a></li>\n",
                        entry.depth.min(4),
                        (self.chapter_page)(n),
                        escape_html(&entry.title)
                    ));
                }
                out.push_str("</ol>\n</nav>\n");
            }
            MatterPage::AboutAuthor(lines) => {
                out.push_str("<section class=\"about-author\">\n");
                heading(out);
                paragraphs(out, lines);
                out.push_str("</section>\n");
            }
            MatterPage::AlsoBy(titles) => {
                out.push_str("<section class=\"also-by\">\n");
                heading(out);
                paragraphs(out, titles);
                out.push_str("</section>\n");
            }
        }
    }

    /// The notes, numbered, each linked back to its mark.
    pub(crate) fn notes_section(&self, heading: &str, level: u8) -> String {
        let kind = if self.xhtml { " epub:type=\"endnotes\"" } else { "" };
        let mut out =
            format!("<section class=\"notes\" role=\"doc-endnotes\"{kind}>\n<h{level}>{}</h{level}>\n<ol>\n", escape_html(heading));
        for (index, (note, page)) in self.notes.iter().enumerate() {
            let n = index + 1;
            let kind = if self.xhtml { " epub:type=\"endnote\"" } else { "" };
            out.push_str(&format!(
                "<li id=\"note-{n}\"{kind}>{} <a href=\"{page}#ref-{n}\" role=\"doc-backlink\" aria-label=\"Back\">↩</a></li>\n",
                escape_html(note)
            ));
        }
        out.push_str("</ol>\n</section>\n");
        out
    }
}

/// The profile's type as CSS. A screen needs more room between lines than
/// Word gives a page, so each spacing opens up a little.
pub(crate) fn type_css(page: &PageSetup) -> String {
    let line = match page.line_spacing {
        LineSpacing::Single => "1.5",
        LineSpacing::OneAndHalf => "1.9",
        LineSpacing::Double => "2.4",
    };
    // A font name is a quoted CSS string: nothing in it may end the string
    // or the style element.
    let font: String = page.font.chars().filter(|ch| !matches!(ch, '"' | '\\' | '<' | '>' | ';' | '{' | '}')).collect();
    let paragraphs = if page.indent {
        "p { margin: 0; }\np + p { text-indent: 1.5em; }"
    } else {
        "p { margin: 0 0 0.9em; }"
    };
    format!(
        r#"body {{ font-family: "{font}", Georgia, "Times New Roman", serif; font-size: {size}pt; line-height: {line}; }}
{paragraphs}
h1, h2, h3, h4, h5, h6 {{ font-weight: 600; line-height: 1.25; margin: 2.2em 0 1em; text-align: center; }}
h1 {{ font-size: 2em; }}
h2 {{ font-size: 1.5em; }}
h3 {{ font-size: 1.25em; }}
h4, h5, h6 {{ font-size: 1em; }}
blockquote {{ margin: 1em 2em; font-style: italic; }}
pre {{ white-space: pre-wrap; font-size: 0.85em; }}
hr {{ border: 0; margin: 1.2em 0; text-align: center; }}
hr::after {{ content: "* * *"; letter-spacing: 0.3em; }}
hr.blank::after {{ content: ""; }}
.title-page {{ text-align: center; margin: 4em 0 5em; }}
.title-page h1 {{ font-size: 2.6em; margin: 0 0 0.4em; }}
.title-page .subtitle {{ font-size: 1.3em; font-style: italic; margin: 0 0 1.5em; text-indent: 0; }}
.title-page .author {{ font-size: 1.15em; text-indent: 0; }}
.copyright {{ font-size: 0.85em; margin: 3em 0; }}
.dedication {{ font-style: italic; text-align: center; margin: 4em 0; }}
.dedication p, .also-by p, .copyright p {{ text-indent: 0; }}
.contents ol {{ list-style: none; padding: 0; }}
.contents li {{ margin: 0.3em 0; }}
.contents .depth-1 {{ padding-left: 1.5em; }}
.contents .depth-2 {{ padding-left: 3em; }}
.contents .depth-3, .contents .depth-4 {{ padding-left: 4.5em; }}
.also-by {{ text-align: center; }}
.also-by p {{ font-style: italic; }}
.note-ref {{ font-size: 0.7em; line-height: 0; }}
.notes {{ font-size: 0.9em; }}
"#,
        size = page.font_size,
    )
}

/// What only the web page has: its own colors and width, a dark scheme, and
/// page breaks when it is printed. An ebook reader sets these itself.
const WEB_CSS: &str = r#"html { background: #fbf8f1; color: #24211c; }
body { margin: 0; }
main { max-width: 34em; margin: 0 auto; padding: 3em 1.25em 5em; }
a { color: #1d4e89; }
.notes { margin-top: 4em; border-top: 1px solid #d9d2c3; }
@media (prefers-color-scheme: dark) {
  html { background: #1d1c1a; color: #e6e0d4; }
  a { color: #8fb4e3; }
  .notes { border-color: #4a463f; }
}
@media print {
  html { background: none; color: #000; }
  main { max-width: none; padding: 0; }
  .title-page, .copyright, .dedication, .contents, .about-author, .also-by, .notes, h1.opens, h2.opens, h3.opens { break-before: page; }
  .title-page { break-before: auto; }
}
"#;
