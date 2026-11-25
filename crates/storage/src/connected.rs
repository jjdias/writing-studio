//! What a binder item is connected to (Phase 15): its own keywords, the
//! notes its context and its text link to with `[[…]]`, and the notes linked
//! to it from Context; for a note, also the items it is linked to. It is
//! written at the top of the item's file, so that whoever reads the file, an
//! AI among them, sees the item's ties before its text, and Context shows the
//! same list.
//!
//! Only the item's own ties: those of the chapter, part, or book it is in
//! are in their own files.

use writing_studio_core::Id;
use writing_studio_format::{same_title, wikilinks};
use writing_studio_project::{BinderNode, BinderTree, NodeKind};

/// One thing an item is connected to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tie {
    /// One of its keywords, as spelled.
    Keyword(String),
    /// A note, by the title a link gives it or, linked from Context, by its
    /// own.
    Note(String),
    /// For a note: an item it is linked to from Context, by its caption
    /// ("Tide / Chapter 1 / Harbor"), since titles repeat.
    Item(String),
}

impl Tie {
    /// The tie as the file lists it: a keyword as it is, a note as
    /// `[[Title]]`, as Obsidian reads a link in front matter.
    pub fn written(&self) -> String {
        match self {
            Tie::Keyword(word) => word.clone(),
            Tie::Note(title) => format!("[[{title}]]"),
            Tie::Item(caption) => caption.clone(),
        }
    }
}

/// The note titles `text` links to, as written, in order.
pub fn links_in(text: &str) -> Vec<String> {
    wikilinks(text).into_iter().map(|link| link.title).collect()
}

/// What `node` is connected to, in order: its keywords, then each note
/// first met in its context, among `text_links` (the titles its own text
/// links to, from [`links_in`]), and among the notes in `tree` linked to it
/// from Context; for a note, then the items it is linked to. Each appears
/// once; nothing in the trash is listed. `node` is passed on its own so that
/// the item as it is now is read, whatever `tree` holds of it.
pub fn connected(tree: &BinderTree, node: &BinderNode, text_links: &[String]) -> Vec<Tie> {
    let id = node.id;
    let mut ties: Vec<Tie> = node.keywords.iter().map(|word| Tie::Keyword(word.clone())).collect();
    let mut notes: Vec<String> = Vec::new();
    let add_note = |title: &str, notes: &mut Vec<String>| {
        let title = title.trim();
        if !title.is_empty() && !notes.iter().any(|kept| same_title(kept, title)) {
            notes.push(title.to_string());
        }
    };
    for title in links_in(&node.context).iter().chain(text_links) {
        add_note(title, &mut notes);
    }
    let live = |other: Id| tree.get(other).is_some_and(|found| found.deleted_at.is_none() && !tree.is_in_trash(other));
    for note in tree.nodes.iter().filter(|other| other.kind == NodeKind::Note && other.links.contains(&id) && live(other.id)) {
        add_note(&note.title, &mut notes);
    }
    ties.extend(notes.into_iter().map(Tie::Note));
    if node.kind == NodeKind::Note {
        for item in node.links.iter().copied().filter(|item| live(*item)) {
            let caption = match tree.get(item) {
                Some(found) if found.kind == NodeKind::Book => found.title.clone(),
                Some(_) => tree.scene_caption(item),
                None => continue,
            };
            let tie = Tie::Item(caption);
            if !ties.contains(&tie) {
                ties.push(tie);
            }
        }
    }
    ties
}

/// What `node`'s file lists: its ties written out, the text's links read
/// from the plain text of `blocks`, as the window reads them for Context.
pub(crate) fn connected_written(tree: &BinderTree, node: &BinderNode, blocks: &[writing_studio_document::Block]) -> Vec<String> {
    let text: Vec<String> = blocks.iter().map(writing_studio_document::Block::plain_text).collect();
    let links = links_in(&text.join("\n"));
    connected(tree, node, &links).iter().map(Tie::written).collect()
}

/// What a file's head lists as `connected`, read without its text.
pub(crate) fn listed_in(file: &str) -> Option<Vec<String>> {
    let file = file.strip_prefix('\u{feff}').unwrap_or(file);
    let (mut header, _) = writing_studio_format::split_header(file).ok()?;
    Some(header.take_list("connected"))
}

/// The part of `node`'s list that comes from the binder rather than its
/// text, written out: what tells that its file's head is out of date when
/// another item changes, without reading its text.
pub(crate) fn binder_ties(tree: &BinderTree, node: &BinderNode) -> Vec<String> {
    connected(tree, node, &[]).iter().map(Tie::written).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_core::Timestamp;

    #[test]
    fn an_item_lists_its_keywords_then_its_notes_once_each() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Tide", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let mut scene = BinderNode::new(NodeKind::Scene, "Harbor", Some(chapter.id), 0);
        scene.keywords = vec!["act one/arrival".into(), "night".into()];
        scene.context = "Mara meets [[The Pilot]].\n\nSee [[mara vance|her]].".into();
        let notes_root = BinderNode::new(NodeKind::NotesRoot, "Notes", None, 1);
        let mut mara = BinderNode::new(NodeKind::Note, "Mara Vance", Some(notes_root.id), 0);
        mara.links = vec![scene.id, chapter.id, book.id];
        let mut port = BinderNode::new(NodeKind::Note, "Port Ely", Some(notes_root.id), 1);
        port.links = vec![scene.id];
        let mut gone = BinderNode::new(NodeKind::Note, "Cut", Some(notes_root.id), 2);
        gone.links = vec![scene.id];
        gone.deleted_at = Some(Timestamp::now());
        let (scene_id, mara_id) = (scene.id, mara.id);
        let tree = BinderTree::new(vec![draft, book, chapter, scene, notes_root, mara, port, gone]);

        let text = links_in("The [[Tide Tables]] say so, and [[The Pilot|he]] agrees.");
        assert_eq!(text, ["Tide Tables", "The Pilot"]);
        let ties = connected(&tree, tree.get(scene_id).unwrap(), &text);
        let written: Vec<String> = ties.iter().map(Tie::written).collect();
        assert_eq!(
            written,
            ["act one/arrival", "night", "[[The Pilot]]", "[[mara vance]]", "[[Tide Tables]]", "[[Port Ely]]"],
            "keywords, then links in the context, in the text, then notes linked from Context; a note in the trash is not"
        );

        let for_note: Vec<String> = connected(&tree, tree.get(mara_id).unwrap(), &[]).iter().map(Tie::written).collect();
        assert_eq!(for_note, ["Tide / Chapter 1 / Harbor", "Tide / Chapter 1", "Tide"], "a note lists what it is linked to");
    }
}
