//! Export must never depend on the editor widget — only on documents and binder order.

mod compile;
mod docx;
mod markdown;

pub use compile::CompileInput;
pub use docx::compile_docx;
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
        let research = BinderNode::new(NodeKind::ResearchRoot, "Research", None, 2);
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
