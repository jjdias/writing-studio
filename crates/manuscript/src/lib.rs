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
                scenes: tree
                    .children(Some(child.id))
                    .into_iter()
                    .filter(|scene| scene.kind == NodeKind::Scene)
                    .collect(),
            }),
            NodeKind::Part => chapters.extend(collect_chapters(tree, child.id)),
            _ => {}
        }
    }
    chapters
}

pub fn default_scene_title(index: usize) -> String {
    format!("Scene {}", index)
}
