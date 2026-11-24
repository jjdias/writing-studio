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

/// The note templates a new project starts with, as files in `templates/`:
/// a title, then headings with a line under each saying what goes there, so
/// a note has a shape a writer, and an AI reading the notes, can follow. The
/// writer changes, adds, or deletes them. A note made from one starts as a
/// copy, with the template's title and keywords.
pub const NOTE_TEMPLATES: [(&str, &str); 4] = [
    (
        "character.md",
        "---\ntitle: New character\ntags:\n  - character\n---\n\n\
Who they are, in a line or two: their place in the story.\n\n\
## What they want\n\nWhat they are after, and what they would never admit to wanting.\n\n\
## What stands in their way\n\nPeople, fears, or flaws that keep them from it.\n\n\
## How they change\n\nWho they are at the start, and at the end.\n\n\
## Details\n\nAppearance, voice, habits, history, and the people around them.\n",
    ),
    (
        "place.md",
        "---\ntitle: New place\ntags:\n  - place\n---\n\n\
What this place is, in a line or two, and where it is.\n\n\
## What happens here\n\nThe scenes set here, and why the story comes here.\n\n\
## The senses\n\nWhat a person sees, hears, smells, and feels here.\n\n\
## Who is here\n\nWho lives or works here, and who comes and goes.\n\n\
## History\n\nWhat happened here before the story, and what is left of it.\n",
    ),
    (
        "source.md",
        "---\ntitle: New source\ntags:\n  - source\n---\n\n\
The citation, as the reader should see it in a footnote: Author, *Title* (Publisher, Year), page.\n\n\
## What it says\n\nThe passage or the facts this source gives. Under a heading, this is never printed.\n\n\
## Why it matters here\n\nWhere it is used in the book, and what it supports.\n",
    ),
    (
        "research.md",
        "---\ntitle: New research note\ntags:\n  - research\n---\n\n\
What this is about, in a line or two.\n\n\
## Key points\n\nWhat was found, one point to a line.\n\n\
## Sources\n\nWhere it came from: books, articles, links, people.\n\n\
## Open questions\n\nWhat is still not known, and where to look.\n",
    ),
];
