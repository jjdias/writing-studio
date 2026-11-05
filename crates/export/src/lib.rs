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
            tree: &tree,
            documents: &docs,
        });
        assert!(markdown.contains("# Test Novel"));
        assert!(markdown.contains("**Hello**"));
        assert!(!markdown.to_lowercase().contains("story notes"));
    }

    #[test]
    fn markdown_exports_links_comments_code_and_scene_breaks() {
        let mut document = Document::new(writing_studio_core::Id::new());
        document.blocks = vec![
            Block {
                id: writing_studio_core::Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![Inline {
                        id: writing_studio_core::Id::new(),
                        text: "harbor".into(),
                        marks: Marks {
                            link: Some("https://example.com/tide".into()),
                            comment: Some("check the tide".into()),
                            ..Marks::default()
                        },
                    }],
                },
            },
            Block {
                id: writing_studio_core::Id::new(),
                kind: BlockKind::Code {
                    inlines: vec![Inline::text("let x = 1;")],
                },
            },
            Block {
                id: writing_studio_core::Id::new(),
                kind: BlockKind::SceneBreak,
            },
        ];
        let markdown = crate::markdown::document_markdown(&document);
        assert!(markdown.contains("[harbor](https://example.com/tide)[^1]"));
        assert!(markdown.contains("[^1]: check the tide"));
        assert!(markdown.contains("```\nlet x = 1;\n```"));
        assert!(markdown.contains("* * *"));
        assert!(!markdown.contains("javascript:"));
    }

    #[test]
    fn docx_bytes_are_a_zip_package() {
        let seeded = blank_novel("Docx Novel");
        let tree = BinderTree::new(seeded.nodes);
        let bytes = compile_docx(&CompileInput {
            title: "Docx Novel",
            include_title_page: true,
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
}
