//! Manuscript-specific operations on the binder.

use writing_studio_core::Id;
use writing_studio_project::{BinderNode, BinderTree, NodeKind};

pub struct ManuscriptOutline<'a> {
    pub books: Vec<OutlineBook<'a>>,
}

pub struct OutlineBook<'a> {
    pub node: &'a BinderNode,
    pub chapters: Vec<OutlineChapter<'a>>,
}

pub struct OutlineChapter<'a> {
    pub node: &'a BinderNode,
    pub scenes: Vec<&'a BinderNode>,
}

pub fn build_outline(tree: &BinderTree) -> ManuscriptOutline<'_> {
    let draft = tree.draft_root();
    let books = tree
        .children(draft.map(|node| node.id))
        .into_iter()
        .filter(|node| node.kind == NodeKind::Book)
        .map(|book| OutlineBook {
            node: book,
            chapters: collect_chapters(tree, book.id),
        })
        .collect();
    ManuscriptOutline { books }
}

fn collect_chapters(tree: &BinderTree, parent: Id) -> Vec<OutlineChapter<'_>> {
    let mut chapters = Vec::new();
    for child in tree.children(Some(parent)) {
        match child.kind {
            NodeKind::Chapter => chapters.push(OutlineChapter {
                node: child,
                scenes: collect_scenes(tree, child.id),
            }),
            NodeKind::Part => chapters.extend(collect_chapters(tree, child.id)),
            NodeKind::Folder => {
                chapters.extend(collect_chapters(tree, child.id));
                let scenes: Vec<&BinderNode> = tree
                    .children(Some(child.id))
                    .into_iter()
                    .filter(|node| node.kind == NodeKind::Scene)
                    .collect();
                if !scenes.is_empty() {
                    chapters.push(OutlineChapter {
                        node: child,
                        scenes,
                    });
                }
            }
            _ => {}
        }
    }
    chapters
}

fn collect_scenes<'a>(tree: &'a BinderTree, parent: Id) -> Vec<&'a BinderNode> {
    let mut scenes = Vec::new();
    for child in tree.children(Some(parent)) {
        match child.kind {
            NodeKind::Scene => scenes.push(child),
            NodeKind::Folder => scenes.extend(collect_scenes(tree, child.id)),
            _ => {}
        }
    }
    scenes
}

pub fn default_scene_title(index: usize) -> String {
    format!("Scene {}", index)
}

/// One outliner row. A chapter with no scenes is a single row for that chapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutlineRow {
    pub book_id: Id,
    pub chapter_id: Id,
    pub scene_id: Option<Id>,
    pub document_id: Option<Id>,
}

pub fn outline_rows(tree: &BinderTree) -> Vec<OutlineRow> {
    let outline = build_outline(tree);
    let mut rows = Vec::new();
    for book in outline.books {
        for chapter in book.chapters {
            if chapter.scenes.is_empty() {
                rows.push(OutlineRow {
                    book_id: book.node.id,
                    chapter_id: chapter.node.id,
                    scene_id: None,
                    document_id: chapter.node.document_id,
                });
            } else {
                for scene in chapter.scenes {
                    rows.push(OutlineRow {
                        book_id: book.node.id,
                        chapter_id: chapter.node.id,
                        scene_id: Some(scene.id),
                        document_id: scene.document_id,
                    });
                }
            }
        }
    }
    rows
}

pub fn row_word_count(document_id: Option<Id>, mut words_for: impl FnMut(Id) -> u32) -> u32 {
    document_id.map(|id| words_for(id)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::BinderNode;

    #[test]
    fn rows_use_scene_documents_and_empty_chapters() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let scene_a = BinderNode::new(NodeKind::Scene, "Scene A", Some(chapter.id), 0);
        let scene_b = BinderNode::new(NodeKind::Scene, "Scene B", Some(chapter.id), 1);
        let empty = BinderNode::new(NodeKind::Chapter, "Chapter 2", Some(book.id), 1);
        let a_doc = scene_a.document_id.unwrap();
        let b_doc = scene_b.document_id.unwrap();
        let empty_doc = empty.document_id.unwrap();
        let tree = BinderTree::new(vec![draft, book, chapter, scene_a, scene_b, empty]);
        let rows = outline_rows(&tree);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].scene_id, Some(tree.nodes[3].id));
        assert_eq!(rows[1].document_id, Some(b_doc));
        assert_eq!(rows[2].scene_id, None);
        assert_eq!(rows[2].chapter_id, tree.nodes[5].id);
        let words = |id: Id| {
            if id == a_doc {
                10
            } else if id == b_doc {
                4
            } else if id == empty_doc {
                2
            } else {
                99
            }
        };
        assert_eq!(row_word_count(rows[0].document_id, words), 10);
        assert_eq!(row_word_count(rows[1].document_id, words), 4);
        assert_eq!(row_word_count(rows[2].document_id, words), 2);
        assert_eq!(row_word_count(None, words), 0);
    }

    #[test]
    fn scenes_inside_folders_stay_on_the_outline() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let group = BinderNode::new(NodeKind::Folder, "Group", Some(chapter.id), 0);
        let scene = BinderNode::new(NodeKind::Scene, "Nested", Some(group.id), 0);
        let scene_id = scene.id;
        let tree = BinderTree::new(vec![draft, book, chapter, group, scene]);
        let rows = outline_rows(&tree);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scene_id, Some(scene_id));
        assert_eq!(rows[0].chapter_id, tree.nodes[2].id);
    }
}
