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
    let notes = BinderNode::new(NodeKind::NotesRoot, "Notes", None, 1);
    let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 2);
    let book = BinderNode::new(NodeKind::Book, title, Some(draft.id), 0);
    let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
    let mut scene = BinderNode::new(NodeKind::Scene, "Scene 1", Some(chapter.id), 0);
    // Two folders to start, the writer's to rename or delete: notes compiled
    // as footnotes where the text links to them, and research that never is.
    let mut footnotes = BinderNode::new(NodeKind::Folder, "Footnotes", Some(notes.id), 0);
    footnotes.footnotes = true;
    let research = BinderNode::new(NodeKind::Folder, "Research", Some(notes.id), 1);

    let scene_doc_id = scene.document_id.unwrap_or_else(Id::new);
    scene.document_id = Some(scene_doc_id);

    // One line is one paragraph: an empty line here would be an empty
    // paragraph in the scene and in every compiled book.
    let mut scene_doc = Document::from_plain_text(
        scene_doc_id,
        "Start writing this scene.\nEvery scene is saved independently.",
    );
    scene_doc.id = scene_doc_id;

    SeededProject {
        meta,
        nodes: vec![draft, notes, trash, book, chapter, scene, footnotes, research],
        documents: vec![scene_doc],
    }
}
