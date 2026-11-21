//! The book as an EPUB 3 ebook: a zip of XHTML pages, one to each part,
//! chapter, and page of matter, with the navigation document, the package
//! that lists them, and an optional cover picture.
//!
//! The pages are written by the same section writer as the web page, as
//! XHTML.

use std::io::Write;

use writing_studio_core::{Id, ImageKind};
use writing_studio_document::escape_html;
use writing_studio_project::NodeKind;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::compile::{collect_sections, CompileInput, CompiledSection};
use crate::html::{book_title, type_css, SectionWriter};

/// What an ebook has that the other formats do not.
pub struct EpubExtras<'a> {
    /// When it was made, as `2026-11-21T09:30:00Z`.
    pub modified: &'a str,
    /// A PNG or JPEG picture for the cover. Anything else is left out.
    pub cover: Option<&'a [u8]>,
}

const NOTES_PAGE: &str = "notes.xhtml";

pub fn compile_epub(input: &CompileInput<'_>, extras: &EpubExtras<'_>) -> std::io::Result<Vec<u8>> {
    let profile = input.profile;
    let language = profile.language;
    let sections = collect_sections(input);
    let title = book_title(&sections, input.title);
    let author = input.author.trim();
    // The book's id stays the same from one compile to the next, so a
    // reader that has the book knows a new compile is the same book.
    let identifier = input.book_id.or_else(|| input.tree.draft_root().map(|draft| draft.id)).unwrap_or_else(Id::new);

    // Which page each section goes on, and the page of each part and
    // chapter, so the contents can link ahead to them.
    let mut page_of = Vec::with_capacity(sections.len());
    let mut pages = 0usize;
    let mut chapter_pages: Vec<String> = Vec::new();
    // Each page of matter is a page of its own, so what follows one starts
    // the next page.
    let mut after_matter = false;
    for section in &sections {
        let starts = pages == 0
            || after_matter
            || section.matter.is_some()
            || section.after_front_matter
            || matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter | NodeKind::Book));
        after_matter = section.matter.is_some();
        if starts {
            pages += 1;
        }
        page_of.push(pages - 1);
        if matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter)) && section.title.is_some() {
            chapter_pages.push(page_name(pages - 1));
        }
    }
    let chapter_page = |n: usize| chapter_pages.get(n - 1).cloned().unwrap_or_default();
    let mut writer = SectionWriter::new(&chapter_page, NOTES_PAGE, true);
    let mut bodies: Vec<String> = vec![String::new(); pages];
    for (section, &page) in sections.iter().zip(&page_of) {
        writer.page = page_name(page);
        writer.section(&mut bodies[page], section);
    }
    let notes = (!writer.notes.is_empty()).then(|| writer.notes_section(language.notes(), 1));

    let cover = extras.cover.and_then(|bytes| ImageKind::of(bytes).map(|kind| (bytes, kind)));
    let lang = language.code();
    let mut book = Book::new()?;
    book.stored("mimetype", b"application/epub+zip")?;
    book.add("META-INF/container.xml", CONTAINER.as_bytes())?;
    book.add("OEBPS/style.css", format!("{}{EBOOK_CSS}", type_css(&profile.page)).as_bytes())?;

    let mut manifest = String::from(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n\
         <item id=\"style\" href=\"style.css\" media-type=\"text/css\"/>\n",
    );
    let mut spine = String::new();
    if let Some((bytes, kind)) = cover {
        let picture = format!("cover.{}", kind.extension());
        book.add(&format!("OEBPS/{picture}"), bytes)?;
        let page = format!("<div class=\"cover\"><img src=\"{picture}\" alt=\"{}\"/></div>\n", escape_html(&title));
        book.add("OEBPS/cover.xhtml", xhtml_page(lang, &title, &page).as_bytes())?;
        manifest.push_str(&format!(
            "<item id=\"cover-image\" href=\"{picture}\" media-type=\"{}\" properties=\"cover-image\"/>\n\
             <item id=\"cover\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            kind.media_type()
        ));
        spine.push_str("<itemref idref=\"cover\" linear=\"no\"/>\n");
    }
    for (index, body) in bodies.iter().enumerate() {
        let name = page_name(index);
        let page_title = page_title(&sections, &page_of, index).unwrap_or_else(|| title.clone());
        book.add(&format!("OEBPS/{name}"), xhtml_page(lang, &page_title, body).as_bytes())?;
        manifest.push_str(&format!("<item id=\"page-{index}\" href=\"{name}\" media-type=\"application/xhtml+xml\"/>\n"));
        spine.push_str(&format!("<itemref idref=\"page-{index}\"/>\n"));
    }
    if let Some(notes) = &notes {
        book.add(&format!("OEBPS/{NOTES_PAGE}"), xhtml_page(lang, language.notes(), notes).as_bytes())?;
        manifest.push_str(&format!("<item id=\"notes\" href=\"{NOTES_PAGE}\" media-type=\"application/xhtml+xml\"/>\n"));
        spine.push_str("<itemref idref=\"notes\"/>\n");
    }
    let contents = navigation(&sections, &chapter_pages);
    let nav_body = format!(
        "<nav epub:type=\"toc\" id=\"toc\">\n<h1>{}</h1>\n{}</nav>\n",
        escape_html(&profile.contents_heading()),
        if contents.is_empty() { format!("<ol>\n<li><a href=\"{}\">{}</a></li>\n</ol>\n", page_name(0), escape_html(&title)) } else { contents }
    );
    book.add("OEBPS/nav.xhtml", xhtml_page(lang, &profile.contents_heading(), &nav_body).as_bytes())?;

    let creator = if author.is_empty() { String::new() } else { format!("<dc:creator>{}</dc:creator>\n", escape_html(author)) };
    let cover_meta = if cover.is_some() { "<meta name=\"cover\" content=\"cover-image\"/>\n" } else { "" };
    let date = extras.modified.get(..10).unwrap_or(extras.modified);
    let package = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\" xml:lang=\"{lang}\">\n\
         <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         <dc:identifier id=\"book-id\">urn:uuid:{identifier}</dc:identifier>\n\
         <dc:title>{title}</dc:title>\n<dc:language>{lang}</dc:language>\n{creator}\
         <dc:date>{date}</dc:date>\n<meta property=\"dcterms:modified\">{modified}</meta>\n{cover_meta}\
         </metadata>\n<manifest>\n{manifest}</manifest>\n<spine>\n{spine}</spine>\n</package>\n",
        title = escape_html(&title),
        date = escape_html(date),
        modified = escape_html(extras.modified),
    );
    book.add("OEBPS/content.opf", xml_text(&package).as_bytes())?;
    book.finish()
}

fn page_name(index: usize) -> String {
    format!("page-{:03}.xhtml", index + 1)
}

/// A page's title: the heading of the first section on it that has one.
fn page_title(sections: &[CompiledSection<'_>], page_of: &[usize], page: usize) -> Option<String> {
    sections.iter().zip(page_of).filter(|(_, &on)| on == page).find_map(|(section, _)| section.title.clone())
}

/// The navigation document's list: the compiled parts and chapters, a
/// chapter inside the part it is in.
fn navigation(sections: &[CompiledSection<'_>], chapter_pages: &[String]) -> String {
    let entries: Vec<(u8, String, String)> = crate::compile::contents_of(sections)
        .into_iter()
        .zip(chapter_pages)
        .enumerate()
        .map(|(index, (entry, page))| (entry.depth, entry.title, format!("{page}#chapter-{}", index + 1)))
        .collect();
    let Some(top) = entries.iter().map(|entry| entry.0).min() else {
        return String::new();
    };
    nested_list(&entries, &mut 0, top)
}

fn nested_list(entries: &[(u8, String, String)], at: &mut usize, depth: u8) -> String {
    let mut out = String::from("<ol>\n");
    while let Some((entry_depth, title, href)) = entries.get(*at) {
        if *entry_depth < depth {
            break;
        }
        *at += 1;
        out.push_str(&format!("<li><a href=\"{href}\">{}</a>", escape_html(title)));
        if entries.get(*at).is_some_and(|next| next.0 > depth) {
            out.push('\n');
            out.push_str(&nested_list(entries, at, depth + 1));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
    out
}

fn xhtml_page(lang: &str, title: &str, body: &str) -> String {
    xml_text(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\">\n\
         <head>\n<meta charset=\"utf-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n</head>\n\
         <body>\n{body}</body>\n</html>\n",
        escape_html(title)
    ))
}

/// Text with the characters XML does not allow taken out. A scene can hold
/// a stray control character that HTML forgives and an ebook reader does not.
fn xml_text(text: &str) -> String {
    text.chars()
        .filter(|&ch| matches!(ch, '\t' | '\n' | '\r') || !(ch < ' ' || ch == '\u{FFFE}' || ch == '\u{FFFF}'))
        .collect()
}

const CONTAINER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
<rootfiles>\n<rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n</rootfiles>\n\
</container>\n";

/// What an ebook adds to the type: the cover fills its page, and each part
/// and chapter has room above it. The reader chooses the colors.
const EBOOK_CSS: &str = ".cover { margin: 0; padding: 0; text-align: center; }
.cover img { max-width: 100%; max-height: 100vh; }
h1.opens, h2.opens, h3.opens { margin-top: 3em; }
";

struct Book {
    zip: ZipWriter<std::io::Cursor<Vec<u8>>>,
}

impl Book {
    fn new() -> std::io::Result<Self> {
        Ok(Self { zip: ZipWriter::new(std::io::Cursor::new(Vec::new())) })
    }

    /// The `mimetype` entry: first, and not compressed, so a reader can tell
    /// what the file is from its first bytes.
    fn stored(&mut self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        self.zip.start_file(name, options).map_err(std::io::Error::other)?;
        self.zip.write_all(bytes)
    }

    fn add(&mut self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        self.zip.start_file(name, options).map_err(std::io::Error::other)?;
        self.zip.write_all(bytes)
    }

    fn finish(self) -> std::io::Result<Vec<u8>> {
        Ok(self.zip.finish().map_err(std::io::Error::other)?.into_inner())
    }
}
