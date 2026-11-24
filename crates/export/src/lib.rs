//! Export must never depend on the editor widget — only on documents and binder order.

mod compile;
mod docx;
mod epub;
mod html;
mod markdown;

pub use compile::CompileInput;
pub use docx::compile_docx;
pub use epub::{compile_epub, EpubExtras};
pub use html::compile_html;
pub use markdown::compile_markdown;

use writing_studio_document::Document;
use writing_studio_project::{BinderTree, CompileProfile, HeadingStyle, SceneSeparator};
#[cfg(test)]
use writing_studio_project::PageSetup;

pub fn manuscript_plain_text(tree: &BinderTree, docs: &[Document]) -> String {
    compile_markdown(&CompileInput {
        title: "Manuscript",
        profile: &as_written(false),
        book_id: None,
        author: "",
        subtitle: "",
        year: 2026,
        tree,
        documents: docs,
    })
}

/// Every heading as it is written in the binder, scenes included: the book
/// as the writer sees it, not as a profile would set it.
fn as_written(title_page: bool) -> CompileProfile {
    let mut profile = writing_studio_project::built_in_profiles().remove(0);
    profile.title_page = title_page;
    profile.chapters = HeadingStyle::Title;
    profile.parts = HeadingStyle::Title;
    profile.scenes = SceneSeparator::Title;
    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_document::{Block, BlockKind, Document, Inline, Marks};
    use writing_studio_project::{BinderNode, BinderTree, NodeKind};
    use writing_studio_templates::blank_novel;

    #[test]
    fn markdown_includes_scene_marks_and_skips_research() {
        let seeded = blank_novel("Test Novel");
        let mut docs = seeded.documents;
        let scene = docs
            .iter_mut()
            .find(|doc| {
                seeded
                    .nodes
                    .iter()
                    .any(|node| node.kind == NodeKind::Scene && node.document_id == Some(doc.id))
            })
            .unwrap();
        scene.blocks = vec![Block {
            id: writing_studio_core::Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![Inline {
                    id: writing_studio_core::Id::new(),
                    text: "Hello".into(),
                    marks: Marks {
                        bold: true,
                        ..Marks::default()
                    },
                }],
            },
        }];
        let tree = BinderTree::new(seeded.nodes);
        let markdown = compile_markdown(&CompileInput {
            title: "Test Novel",
            profile: &as_written(true),
            book_id: None,
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &docs,
        });
        assert!(markdown.contains("# Test Novel"));
        assert!(markdown.contains("**Hello**"));
        assert!(!markdown.to_lowercase().contains("story notes"));
    }

    /// Compile used to number footnotes from 1 in every scene, so a book had
    /// several `[^1]`. They are numbered across the book now.
    #[test]
    fn markdown_numbers_footnotes_across_the_book_and_escapes_titles() {
        let seeded = blank_novel("Notes *Novel*");
        let mut tree = BinderTree::new(seeded.nodes);
        let chapter = tree.nodes.iter().find(|n| n.kind == NodeKind::Chapter).unwrap().id;
        let first = tree.nodes.iter().find(|n| n.kind == NodeKind::Scene).unwrap().clone();
        let mut second = BinderNode::new(NodeKind::Scene, "# Second", Some(chapter), 1);
        second.document_id = Some(second.id);
        tree.nodes.push(second.clone());
        let noted = |id, text: &str, note: &str| {
            let mut document = Document::new(id);
            document.blocks = vec![Block {
                id: writing_studio_core::Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![Inline {
                        id: writing_studio_core::Id::new(),
                        text: text.into(),
                        marks: Marks {
                            comment: Some(note.into()),
                            link: Some("https://example.com/tide".into()),
                            ..Marks::default()
                        },
                    }],
                },
            }];
            document
        };
        let documents = vec![
            noted(first.document_id.unwrap(), "harbor", "check the tide"),
            noted(second.id, "quay", "which one?"),
        ];
        let markdown = compile_markdown(&CompileInput {
            title: "Notes *Novel*",
            profile: &as_written(true),
            book_id: None,
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &documents,
        });
        assert!(markdown.starts_with("# Notes \\*Novel\\*\n"), "{markdown}");
        assert!(markdown.contains("### \\# Second\n"), "{markdown}");
        assert!(markdown.contains("[harbor](https://example.com/tide)[^1]"), "{markdown}");
        assert!(markdown.contains("[quay](https://example.com/tide)[^2]"), "{markdown}");
        assert!(markdown.ends_with("[^1]: check the tide\n\n[^2]: which one?\n"), "{markdown}");
    }

    #[test]
    fn docx_bytes_are_a_zip_package() {
        let seeded = blank_novel("Docx Novel");
        let tree = BinderTree::new(seeded.nodes);
        let bytes = compile_docx(&CompileInput {
            title: "Docx Novel",
            profile: &as_written(true),
            book_id: None,
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &seeded.documents,
        })
        .unwrap();
        assert!(bytes.starts_with(b"PK"));
        assert!(bytes.len() > 100);
    }

    #[test]
    fn docx_takes_the_page_from_the_profile() {
        use std::io::Read;
        use writing_studio_project::{LineSpacing, PageSize};
        let seeded = blank_novel("Paged Novel");
        let tree = BinderTree::new(seeded.nodes);
        let page = PageSetup {
            font: "Garamond".into(),
            font_size: 11,
            line_spacing: LineSpacing::Double,
            size: PageSize::A4,
            margin: 12,
            indent: true,
            chapter_new_page: true,
        };
        let bytes = compile_docx(&CompileInput {
            title: "Paged Novel",
            profile: &CompileProfile { page, ..as_written(true) },
            book_id: None,
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &seeded.documents,
        })
        .unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains(r#"w:w="11906""#) && xml.contains(r#"w:h="16838""#), "A4");
        assert!(xml.contains(r#"w:top="1728""#), "1.2 inch margins");
        assert!(xml.contains(r#"w:line="480""#), "double spaced");
        assert!(xml.contains(r#"w:firstLine="720""#), "indented paragraphs");
        assert!(xml.contains("Garamond"));
        assert!(xml.contains("<w:sz w:val=\"22\""), "11 point text");
        assert!(xml.contains("w:pageBreakBefore"), "the text after the title page starts a page");
    }

    #[test]
    fn draft_only_walk_uses_binder_order() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 1);
        let research = BinderNode::new(NodeKind::NotesRoot, "Research", None, 2);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Ch 1", Some(book.id), 0);
        let scene = BinderNode::new(NodeKind::Scene, "Scene", Some(chapter.id), 0);
        let doc_id = scene.document_id.unwrap();
        let document = Document::from_plain_text(doc_id, "Body text.");
        let tree = BinderTree::new(vec![draft, trash, research, book, chapter, scene]);
        let markdown = manuscript_plain_text(&tree, &[document]);
        assert!(markdown.contains("Body text."));
        assert!(markdown.contains("Scene"));
        assert!(!markdown.contains("Research"));
    }

    #[test]
    fn compile_one_book_leaves_the_other_out() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let first = BinderNode::new(NodeKind::Book, "Harbor", Some(draft.id), 0);
        let second = BinderNode::new(NodeKind::Book, "Inland", Some(draft.id), 1);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(first.id), 0);
        let scene = BinderNode::new(NodeKind::Scene, "Tide", Some(chapter.id), 0);
        let other_chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(second.id), 0);
        let other = BinderNode::new(NodeKind::Scene, "Ridge", Some(other_chapter.id), 0);
        let tide = Document::from_plain_text(scene.document_id.unwrap(), "Salt on the pier.");
        let ridge = Document::from_plain_text(other.document_id.unwrap(), "Dust on the road.");
        let first_id = first.id;
        let tree = BinderTree::new(vec![draft, first, second, chapter, scene, other_chapter, other]);
        let markdown = compile_markdown(&CompileInput {
            title: "Series",
            profile: &as_written(true),
            book_id: Some(first_id),
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &[tide, ridge],
        });
        assert!(markdown.contains("# Harbor"));
        assert!(markdown.contains("Salt on the pier."));
        assert!(!markdown.contains("# Series"));
        assert!(!markdown.contains("Inland"));
        assert!(!markdown.contains("Dust on the road."));
        assert!(!markdown.contains("Ridge"));
    }

    /// Every element the page opens is closed, in order. Void elements
    /// (`<br>`, `<hr>`, `<meta>`) close themselves.
    fn assert_balanced(html: &str) {
        const VOID: [&str; 5] = ["br", "hr", "meta", "img", "link"];
        let mut open: Vec<String> = Vec::new();
        let mut rest = html;
        while let Some(at) = rest.find('<') {
            rest = &rest[at + 1..];
            let end = rest.find('>').expect("a tag is closed");
            let tag = &rest[..end];
            rest = &rest[end + 1..];
            if tag.starts_with('!') {
                continue;
            }
            let name: String = tag.trim_start_matches('/').chars().take_while(|ch| ch.is_ascii_alphanumeric()).collect();
            if let Some(closing) = tag.strip_prefix('/') {
                let closing: String = closing.chars().take_while(|ch| ch.is_ascii_alphanumeric()).collect();
                assert_eq!(open.pop().as_deref(), Some(closing.as_str()), "closed in order, before {rest:.80}");
            } else if !VOID.contains(&name.as_str()) {
                open.push(name);
            }
        }
        assert!(open.is_empty(), "left open: {open:?}");
    }

    #[test]
    fn epub_is_a_well_formed_package_with_mimetype_first_and_every_file_it_lists() {
        use std::collections::HashMap;
        use std::io::Read;
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Salt & <Smoke>", Some(draft.id), 0);
        let part = BinderNode::new(NodeKind::Part, "Sea", Some(book.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "The Pier", Some(part.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Inland", Some(part.id), 1);
        let dawn = BinderNode::new(NodeKind::Scene, "Dawn", Some(one.id), 0);
        let noon = BinderNode::new(NodeKind::Scene, "Noon", Some(one.id), 1);
        let road = BinderNode::new(NodeKind::Scene, "Road", Some(two.id), 0);
        let mut first = Document::new(dawn.document_id.unwrap());
        first.blocks = vec![Block {
            id: writing_studio_core::Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![Inline {
                    id: writing_studio_core::Id::new(),
                    text: "Salt\u{7} on the\npier & <rope>".into(),
                    marks: Marks { comment: Some("check the tide".into()), ..Marks::default() },
                }],
            },
        }];
        let second = Document::from_plain_text(noon.document_id.unwrap(), "High sun.");
        let third = Document::from_plain_text(road.document_id.unwrap(), "Dust.");
        let book_id = book.id;
        let tree = BinderTree::new(vec![draft, book, part, one, two, dawn, noon, road]);
        let mut profile = writing_studio_project::built_in_profiles().remove(0);
        profile.matter.contents = true;
        profile.matter.dedication = true;
        profile.matter.dedication_text = "For Ana".into();
        profile.matter.about_author = true;
        profile.matter.about_author_text = "Maya lives by the sea.".into();
        let input = CompileInput {
            title: "Series",
            book_id: Some(book_id),
            author: "Maya Reis",
            subtitle: "A Novel",
            year: 2026,
            tree: &tree,
            documents: &[first, second, third],
            profile: &profile,
        };
        let png = b"\x89PNG\r\n\x1a\nnot really a picture".as_slice();
        let bytes =
            compile_epub(&input, &EpubExtras { modified: "2026-11-21T09:30:00Z", cover: Some(png) }).unwrap();

        // `mimetype` first, stored, with nothing between its name and its
        // text, so a reader knows the file from its first bytes.
        assert_eq!(&bytes[30..38], b"mimetype");
        assert_eq!(&bytes[38..58], b"application/epub+zip");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(archive.by_index(0).unwrap().compression(), zip::CompressionMethod::Stored);
        let mut files = HashMap::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let mut content = Vec::new();
            entry.read_to_end(&mut content).unwrap();
            files.insert(entry.name().to_string(), content);
        }
        let text = |name: &str| String::from_utf8(files[name].clone()).unwrap();
        for (name, content) in &files {
            if [".xhtml", ".opf", ".xml"].iter().any(|ending| name.ends_with(ending)) {
                let content = std::str::from_utf8(content).unwrap();
                let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
                roxmltree::Document::parse_with_options(content, options).unwrap_or_else(|err| panic!("{name}: {err}\n{content}"));
            }
        }

        let package_text = text("OEBPS/content.opf");
        let package = roxmltree::Document::parse(&package_text).unwrap();
        let items: HashMap<&str, &str> = package
            .descendants()
            .filter(|node| node.has_tag_name("item"))
            .map(|node| (node.attribute("id").unwrap(), node.attribute("href").unwrap()))
            .collect();
        for href in items.values() {
            assert!(files.contains_key(&format!("OEBPS/{href}")), "{href} is in the book");
        }
        let spine: Vec<&str> =
            package.descendants().filter(|node| node.has_tag_name("itemref")).map(|node| node.attribute("idref").unwrap()).collect();
        assert!(spine.iter().all(|id| items.contains_key(id)), "{spine:?}");
        assert_eq!(spine[0], "cover", "the cover comes first");
        assert!(package_text.contains(&format!("urn:uuid:{book_id}")), "the book's own id");
        assert!(package_text.contains("<dc:title>Salt &amp; &lt;Smoke&gt;</dc:title>"));
        assert!(package_text.contains("<dc:creator>Maya Reis</dc:creator>"));
        assert!(package_text.contains("<meta property=\"dcterms:modified\">2026-11-21T09:30:00Z</meta>"));
        assert!(files.contains_key("OEBPS/cover.png"));

        // A page each for the matter, the part, and each chapter, in order.
        let page = |n: usize| text(&format!("OEBPS/page-{n:03}.xhtml"));
        assert!(page(1).contains("class=\"title-page\"") && page(2).contains("For Ana") && page(3).contains("Contents"));
        assert!(page(4).contains("Part 1: Sea") && !page(4).contains("Chapter 1"));
        assert!(page(5).contains("Chapter 1: The Pier") && page(5).contains("High sun.") && page(5).contains("<hr/>"));
        assert!(page(6).contains("Chapter 2: Inland") && page(7).contains("About the author"));
        assert!(page(5).contains("Salt on the<br/>pier &amp; &lt;rope&gt;"), "{}", page(5));
        assert!(page(3).contains("href=\"page-005.xhtml#chapter-2\""), "the contents link ahead: {}", page(3));

        // The navigation nests a chapter in its part; a note links both ways.
        let nav = text("OEBPS/nav.xhtml");
        assert!(nav.contains("Part 1: Sea</a>\n<ol>\n<li><a href=\"page-005.xhtml#chapter-2\">"), "{nav}");
        assert!(page(5).contains("<a id=\"ref-1\" href=\"notes.xhtml#note-1\" role=\"doc-noteref\" epub:type=\"noteref\">1</a>"));
        assert!(text("OEBPS/notes.xhtml").contains("check the tide <a href=\"page-005.xhtml#ref-1\""));

        // A picture that is not PNG or JPEG is left out, and the book is whole without it.
        let bytes = compile_epub(&input, &EpubExtras { modified: "2026-11-21T09:30:00Z", cover: Some(b"<svg/>") }).unwrap();
        let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert!(!archive.file_names().any(|name| name.contains("cover")));
    }

    #[test]
    fn html_is_one_well_formed_escaped_page_with_matter_contents_and_notes_both_ways() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Salt & <Smoke>", Some(draft.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "The \"Pier\"", Some(book.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Inland", Some(book.id), 1);
        let dawn = BinderNode::new(NodeKind::Scene, "Dawn", Some(one.id), 0);
        let noon = BinderNode::new(NodeKind::Scene, "Noon", Some(one.id), 1);
        let road = BinderNode::new(NodeKind::Scene, "Road", Some(two.id), 0);
        let mut first = Document::new(dawn.document_id.unwrap());
        first.blocks = vec![Block {
            id: writing_studio_core::Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![Inline {
                    id: writing_studio_core::Id::new(),
                    text: "<script>alert(1)</script> tide".into(),
                    marks: Marks { comment: Some("check </li> & tide".into()), ..Marks::default() },
                }],
            },
        }];
        let second = Document::from_plain_text(noon.document_id.unwrap(), "High sun.");
        let third = Document::from_plain_text(road.document_id.unwrap(), "Dust.");
        let book_id = book.id;
        let tree = BinderTree::new(vec![draft, book, one, two, dawn, noon, road]);
        let mut profile = writing_studio_project::built_in_profiles().remove(2);
        assert_eq!(profile.format, writing_studio_project::CompileFormat::Html, "the Web page profile");
        profile.page.font = "Evil\"; } body { color: red".into();
        profile.matter.dedication = true;
        profile.matter.dedication_text = "For <Ana>".into();
        let html = compile_html(&CompileInput {
            title: "Series",
            book_id: Some(book_id),
            author: "Maya \"M\" Reis",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &[first, second, third],
            profile: &profile,
        });
        assert_balanced(&html);
        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"), "{html:.80}");
        assert!(html.contains("<title>Salt &amp; &lt;Smoke&gt;</title>"));
        assert!(html.contains("<meta name=\"author\" content=\"Maya &quot;M&quot; Reis\">"));
        assert!(!html.contains("<script"), "text is never markup: {html}");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt; tide"));
        assert!(html.contains("<p>For &lt;Ana&gt;</p>"));
        assert!(html.contains("font-family: \"Evil  body  color: red\""), "a font name cannot leave its string");
        // The contents link to the chapters, and each note links both ways.
        assert!(html.contains("<a href=\"#chapter-1\">Chapter 1: The &quot;Pier&quot;</a>"), "{html}");
        assert!(html.contains("<h2 id=\"chapter-2\" class=\"opens\">Chapter 2: Inland</h2>"), "{html}");
        assert!(html.contains("<a id=\"ref-1\" href=\"#note-1\" role=\"doc-noteref\">1</a>"));
        assert!(html.contains("<li id=\"note-1\">check &lt;/li&gt; &amp; tide <a href=\"#ref-1\""));
        let first_text = html.find("tide").unwrap();
        let noon_text = html.find("High sun.").unwrap();
        assert!(html[first_text..noon_text].contains("<hr>"), "a scene break between scenes");
        assert!(html.find("<h2>Notes</h2>").unwrap() > html.find("Dust.").unwrap(), "notes at the end");
    }

    /// A new chapter's own text is one empty paragraph. Compile wrote it as
    /// an empty line under every such heading, in Word, EPUB, and HTML.
    #[test]
    fn a_heading_whose_text_is_empty_has_no_empty_paragraph_under_it() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Tide", Some(draft.id), 0);
        let empty = BinderNode::new(NodeKind::Chapter, "Empty", Some(book.id), 0);
        let written = BinderNode::new(NodeKind::Chapter, "Written", Some(book.id), 1);
        let dawn = BinderNode::new(NodeKind::Scene, "Dawn", Some(empty.id), 0);
        let docs = [
            Document::new(empty.document_id.unwrap()),
            Document::from_plain_text(written.document_id.unwrap(), "A chapter's own words."),
            // An empty line the writer left inside a scene stays.
            Document::from_plain_text(dawn.document_id.unwrap(), "One.\n\nTwo."),
        ];
        let book_id = book.id;
        let tree = BinderTree::new(vec![draft, book, empty, written, dawn]);
        let profile = writing_studio_project::built_in_profiles().remove(2);
        let html = compile_html(&CompileInput {
            title: "Tide",
            book_id: Some(book_id),
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &docs,
            profile: &profile,
        });
        let heading = html.find("Empty</h2>").unwrap();
        let scene = html.find("<p>One.</p>").unwrap();
        assert!(!html[heading..scene].contains("<p>"), "nothing between the heading and its scene: {html}");
        assert!(html.contains("<p>One.</p><p><br></p><p>Two.</p>"), "{html}");
        assert!(html.contains("<p>A chapter's own words.</p>"), "a chapter's own text stays: {html}");
    }

    #[test]
    fn front_and_back_matter_go_around_the_text_in_order_and_empty_pages_are_left_out() {
        use std::io::Read;
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Harbor", Some(draft.id), 0);
        let part = BinderNode::new(NodeKind::Part, "Sea", Some(book.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "The Pier", Some(part.id), 0);
        let cut = BinderNode::new(NodeKind::Chapter, "Cut", Some(part.id), 1);
        let scene = BinderNode::new(NodeKind::Scene, "Dawn", Some(one.id), 0);
        let text = Document::from_plain_text(scene.document_id.unwrap(), "Salt on the pier.");
        let mut cut = cut;
        cut.left_out = true;
        let book_id = book.id;
        let tree = BinderTree::new(vec![draft, book, part, one, cut, scene]);
        let mut profile = writing_studio_project::built_in_profiles().remove(1);
        let matter = &mut profile.matter;
        matter.copyright = true;
        matter.dedication = true;
        matter.dedication_text = "For Ana,\nwho waited.".into();
        matter.contents = true;
        matter.about_author = true;
        matter.about_author_text = "Maya lives by the sea.\n\nShe writes at night.".into();
        matter.also_by = true;
        matter.also_by_text = "Inland\n\nThe Ridge\n".into();
        let input = |profile: &CompileProfile, author: &str, docx: bool| {
            let input = CompileInput {
                title: "Harbor",
                book_id: Some(book_id),
                author,
                subtitle: "A Novel",
                year: 2026,
                tree: &tree,
                documents: std::slice::from_ref(&text),
                profile,
            };
            if docx { compile_docx(&input).unwrap() } else { compile_markdown(&input).into_bytes() }
        };
        let markdown_of = |profile: &CompileProfile, author: &str| String::from_utf8(input(profile, author, false)).unwrap();
        let markdown = markdown_of(&profile, "Maya Reis");
        let order = [
            "# Harbor\n",
            "*A Novel*",
            "Maya Reis",
            "Copyright © 2026 Maya Reis. All rights reserved.",
            "*For Ana,*",
            "*who waited.*",
            "## Contents",
            "- Part 1: Sea",
            "- – Chapter 1: The Pier",
            "## Part 1: Sea",
            "Salt on the pier.",
            "## About the author",
            "She writes at night.",
            "## Also by Maya Reis",
            "- The Ridge",
        ];
        let mut from = 0;
        for piece in order {
            let at = markdown[from..].find(piece).unwrap_or_else(|| panic!("{piece:?} after {from} in {markdown}"));
            from += at + piece.len();
        }
        assert!(!markdown.contains("Cut"), "a chapter left out is not in the contents: {markdown}");

        // With nothing to say, a page is left out, and no author leaves no gap.
        profile.matter.dedication_text = "  ".into();
        profile.matter.also_by_text = String::new();
        let markdown = markdown_of(&profile, "");
        assert!(markdown.contains("Copyright © 2026. All rights reserved."), "{markdown}");
        assert!(!markdown.contains("For Ana") && !markdown.contains("Also by"), "{markdown}");

        // In Word each page of matter starts a page of its own.
        let bytes = input(&profile, "Maya Reis", true);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        for piece in ["Harbor", "A Novel", "Copyright © 2026 Maya Reis", "Contents", "Salt on the pier.", "About the author"] {
            assert!(xml.contains(piece), "{piece} in the DOCX");
        }
        // The copyright page, the contents, the text, and the back matter.
        assert!(xml.matches("w:pageBreakBefore").count() >= 4, "{}", xml.matches("w:pageBreakBefore").count());
    }

    #[test]
    fn a_profile_numbers_chapters_breaks_between_scenes_and_leaves_out_what_is_left_out() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Harbor", Some(draft.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let skipped = BinderNode::new(NodeKind::Chapter, "Notes to self", Some(book.id), 1);
        let two = BinderNode::new(NodeKind::Chapter, "The Ridge", Some(book.id), 2);
        let scene = |title: &str, parent: &BinderNode, at| BinderNode::new(NodeKind::Scene, title, Some(parent.id), at);
        let (a, b, c) = (scene("Dawn", &one, 0), scene("Noon", &one, 1), scene("Dusk", &one, 2));
        let d = scene("Climb", &two, 0);
        let hidden = scene("Cut", &skipped, 0);
        let text = |node: &BinderNode, words: &str| Document::from_plain_text(node.document_id.unwrap(), words);
        let documents =
            vec![text(&a, "First light."), text(&b, "High sun."), text(&c, "Last light."), text(&d, "Up."), text(&hidden, "Gone.")];
        let mut c = c;
        c.left_out = true;
        let mut skipped = skipped;
        skipped.left_out = true;
        let book_id = book.id;
        let tree = BinderTree::new(vec![draft, book, one, skipped, two, a, b, c, d, hidden]);
        let mut profile = writing_studio_project::built_in_profiles().remove(1);
        profile.title_page = false;
        let markdown = compile_markdown(&CompileInput {
            title: "Harbor",
            book_id: Some(book_id),
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &documents,
            profile: &profile,
        });
        assert!(markdown.contains("# Chapter 1\n"), "a placeholder title is not repeated: {markdown}");
        assert!(markdown.contains("# Chapter 2: The Ridge\n"), "numbered among the compiled chapters: {markdown}");
        assert!(!markdown.contains("Dawn") && !markdown.contains("Noon"), "no scene titles: {markdown}");
        let first = markdown.find("First light.").unwrap();
        let second = markdown.find("High sun.").unwrap();
        assert!(markdown[first..second].contains("* * *"), "a break between scenes: {markdown}");
        let climb = markdown.find("Up.").unwrap();
        let heading = markdown.find("Chapter 2").unwrap();
        assert!(!markdown[heading..climb].contains("* * *"), "none before a chapter's first scene: {markdown}");
        assert!(!markdown.contains("Last light."), "a scene left out: {markdown}");
        assert!(!markdown.contains("Gone.") && !markdown.contains("Notes to self"), "a chapter left out, with its scenes");

        profile.chapters = HeadingStyle::NumberWords;
        profile.chapter_word = "Capítulo".into();
        profile.scenes = SceneSeparator::Title;
        let markdown = compile_markdown(&CompileInput {
            title: "Harbor",
            book_id: Some(book_id),
            author: "",
            subtitle: "",
            year: 2026,
            tree: &tree,
            documents: &documents,
            profile: &profile,
        });
        assert!(markdown.contains("Capítulo Two"), "{markdown}");
        assert!(markdown.contains("Noon") && !markdown.contains("* * *"), "{markdown}");
    }
}
