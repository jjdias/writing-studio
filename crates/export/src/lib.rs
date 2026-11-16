//! Export must never depend on the editor widget — only on documents and binder order.

mod compile;
mod docx;
mod markdown;

pub use compile::CompileInput;
pub use docx::compile_docx;
pub use markdown::compile_markdown;

use writing_studio_document::Document;
use writing_studio_project::BinderTree;

pub fn manuscript_plain_text(tree: &BinderTree, docs: &[Document]) -> String {
    compile_markdown(&CompileInput {
        title: "Manuscript",
        include_title_page: false,
        book_id: None,
        tree,
        documents: docs,
    })
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
            include_title_page: true,
            book_id: None,
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
            include_title_page: true,
            book_id: None,
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
            include_title_page: true,
            book_id: None,
            tree: &tree,
            documents: &seeded.documents,
        })
        .unwrap();
        assert!(bytes.starts_with(b"PK"));
        assert!(bytes.len() > 100);
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
            include_title_page: true,
            book_id: Some(first_id),
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
}
