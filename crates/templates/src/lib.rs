//! Built-in project templates. Plugins can add more later without changing core.

use writing_studio_core::Id;
use writing_studio_document::Document;
use writing_studio_project::{BinderNode, NodeKind, ProjectMeta};

pub struct SeededProject {
    pub meta: ProjectMeta,
    pub nodes: Vec<BinderNode>,
    pub documents: Vec<Document>,
}

pub fn blank_novel(title: &str) -> SeededProject {
    let meta = ProjectMeta::new(title);
    let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
    let research = BinderNode::new(NodeKind::ResearchRoot, "Research", None, 1);
    let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 2);
    let book = BinderNode::new(NodeKind::Book, title, Some(draft.id), 0);
    let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
    let mut scene = BinderNode::new(NodeKind::Scene, "Scene 1", Some(chapter.id), 0);
    let mut note = BinderNode::new(NodeKind::Note, "Story notes", Some(research.id), 0);

    let scene_doc_id = scene.document_id.unwrap_or_else(Id::new);
    scene.document_id = Some(scene_doc_id);
    let note_doc_id = note.document_id.unwrap_or_else(Id::new);
    note.document_id = Some(note_doc_id);

    let mut scene_doc = Document::from_plain_text(
        scene_doc_id,
        "Start writing this scene.\n\nEvery scene is saved independently.",
    );
    scene_doc.id = scene_doc_id;
    let mut note_doc = Document::from_plain_text(
        note_doc_id,
        "Research and notes live beside the manuscript, not inside it.",
    );
    note_doc.id = note_doc_id;

    SeededProject {
        meta,
        nodes: vec![draft, research, trash, book, chapter, scene, note],
        documents: vec![scene_doc, note_doc],
    }
}
