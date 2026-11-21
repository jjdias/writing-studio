//! The book as one web page: its text, its matter, its notes at the end, and
//! a stylesheet from the profile, all in the one file. Nothing is loaded
//! from anywhere else and nothing runs, so it opens in any browser and can
//! be put on a website as it is.

use writing_studio_document::escape_html;
use writing_studio_project::{LineSpacing, NodeKind, PageSetup, SceneSeparator};

use crate::compile::{collect_sections, CompileInput, MatterPage};

pub fn compile_html(input: &CompileInput<'_>) -> String {
    let profile = input.profile;
    let sections = collect_sections(input);
    // The title is the title page's, which is the book's own when one book
    // is compiled.
    let title = sections
        .iter()
        .find(|section| matches!(section.matter, Some(MatterPage::Title { .. })))
        .and_then(|section| section.title.clone())
        .unwrap_or_else(|| input.title.to_string());

    let mut body = String::new();
    let mut notes: Vec<String> = Vec::new();
    // Parts and chapters, counted in the order the contents lists them.
    let mut headed = 0;
    for section in &sections {
        match section.break_before {
            Some(SceneSeparator::Break) => body.push_str("<hr>\n"),
            Some(SceneSeparator::Blank) => body.push_str("<hr class=\"blank\">\n"),
            _ => {}
        }
        if let Some(matter) = &section.matter {
            write_matter(&mut body, matter, section.title.as_deref(), section.heading_level);
            continue;
        }
        if let Some(heading) = &section.title {
            let level = section.heading_level.clamp(1, 6);
            let opens = matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter | NodeKind::Book));
            let anchor = if matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter)) {
                headed += 1;
                format!(" id=\"chapter-{headed}\"")
            } else {
                String::new()
            };
            let class = if opens { " class=\"opens\"" } else { "" };
            body.push_str(&format!("<h{level}{anchor}{class}>{}</h{level}>\n", escape_html(heading)));
        }
        if let Some(document) = section.document {
            let text = document.to_reader_html(&mut |note| {
                notes.push(note.to_string());
                let n = notes.len();
                format!("<sup class=\"note-ref\"><a id=\"ref-{n}\" href=\"#note-{n}\" role=\"doc-noteref\">{n}</a></sup>")
            });
            body.push_str(&text);
            body.push('\n');
        }
    }
    if !notes.is_empty() {
        body.push_str(&format!(
            "<section class=\"notes\" role=\"doc-endnotes\">\n<h2>{}</h2>\n<ol>\n",
            escape_html(profile.language.notes())
        ));
        for (index, note) in notes.iter().enumerate() {
            let n = index + 1;
            body.push_str(&format!(
                "<li id=\"note-{n}\">{} <a href=\"#ref-{n}\" role=\"doc-backlink\" aria-label=\"Back\">↩</a></li>\n",
                escape_html(note)
            ));
        }
        body.push_str("</ol>\n</section>\n");
    }

    let author = input.author.trim();
    let author_meta =
        if author.is_empty() { String::new() } else { format!("<meta name=\"author\" content=\"{}\">\n", escape_html(author)) };
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"Writing Studio\">\n{author_meta}<title>{title}</title>\n\
         <style>\n{style}</style>\n</head>\n<body>\n<main>\n{body}</main>\n</body>\n</html>\n",
        lang = profile.language.code(),
        title = escape_html(&title),
        style = stylesheet(&profile.page),
    )
}

fn write_matter(body: &mut String, matter: &MatterPage, title: Option<&str>, level: u8) {
    let level = level.clamp(1, 6);
    let heading = |body: &mut String| {
        if let Some(title) = title {
            body.push_str(&format!("<h{level}>{}</h{level}>\n", escape_html(title)));
        }
    };
    let paragraphs = |body: &mut String, lines: &[String]| {
        for line in lines {
            body.push_str(&format!("<p>{}</p>\n", escape_html(line)));
        }
    };
    match matter {
        MatterPage::Title { subtitle, author } => {
            body.push_str("<header class=\"title-page\">\n");
            if let Some(title) = title {
                body.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));
            }
            if !subtitle.is_empty() {
                body.push_str(&format!("<p class=\"subtitle\">{}</p>\n", escape_html(subtitle)));
            }
            if !author.is_empty() {
                body.push_str(&format!("<p class=\"author\">{}</p>\n", escape_html(author)));
            }
            body.push_str("</header>\n");
        }
        MatterPage::Copyright(line) => {
            body.push_str(&format!("<section class=\"copyright\">\n<p>{}</p>\n</section>\n", escape_html(line)));
        }
        MatterPage::Dedication(lines) => {
            body.push_str("<section class=\"dedication\">\n");
            paragraphs(body, lines);
            body.push_str("</section>\n");
        }
        MatterPage::Contents(entries) => {
            body.push_str("<nav class=\"contents\" role=\"doc-toc\">\n");
            heading(body);
            body.push_str("<ol>\n");
            for (index, entry) in entries.iter().enumerate() {
                body.push_str(&format!(
                    "<li class=\"depth-{}\"><a href=\"#chapter-{}\">{}</a></li>\n",
                    entry.depth.min(4),
                    index + 1,
                    escape_html(&entry.title)
                ));
            }
            body.push_str("</ol>\n</nav>\n");
        }
        MatterPage::AboutAuthor(lines) => {
            body.push_str("<section class=\"about-author\">\n");
            heading(body);
            paragraphs(body, lines);
            body.push_str("</section>\n");
        }
        MatterPage::AlsoBy(titles) => {
            body.push_str("<section class=\"also-by\">\n");
            heading(body);
            paragraphs(body, titles);
            body.push_str("</section>\n");
        }
    }
}

/// The profile's type as CSS. A screen needs more room between lines than
/// Word gives a page, so each spacing opens up a little.
fn stylesheet(page: &PageSetup) -> String {
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
        r#"html {{ background: #fbf8f1; color: #24211c; }}
body {{ margin: 0; font-family: "{font}", Georgia, "Times New Roman", serif; font-size: {size}pt; line-height: {line}; }}
main {{ max-width: 34em; margin: 0 auto; padding: 3em 1.25em 5em; }}
{paragraphs}
h1, h2, h3, h4, h5, h6 {{ font-weight: 600; line-height: 1.25; margin: 2.2em 0 1em; text-align: center; }}
h1 {{ font-size: 2em; }}
h2 {{ font-size: 1.5em; }}
h3 {{ font-size: 1.25em; }}
h4, h5, h6 {{ font-size: 1em; }}
blockquote {{ margin: 1em 2em; font-style: italic; }}
pre {{ white-space: pre-wrap; font-size: 0.85em; }}
a {{ color: #1d4e89; }}
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
.notes {{ font-size: 0.9em; margin-top: 4em; border-top: 1px solid #d9d2c3; }}
@media (prefers-color-scheme: dark) {{
  html {{ background: #1d1c1a; color: #e6e0d4; }}
  a {{ color: #8fb4e3; }}
  .notes {{ border-color: #4a463f; }}
}}
@media print {{
  html {{ background: none; color: #000; }}
  main {{ max-width: none; padding: 0; }}
  .title-page, .copyright, .dedication, .contents, .about-author, .also-by, .notes, h1.opens, h2.opens, h3.opens {{ break-before: page; }}
  .title-page {{ break-before: auto; }}
}}
"#,
        size = page.font_size,
    )
}
