//! A book as a folder of Markdown files, for reading it, or for taking it to
//! another writing program; and Markdown files brought in as scenes.
//!
//! The export is for leaving, not for coming back: ids of plot points,
//! settings, and this machine's files are not written. The project's own
//! files stay the project.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use writing_studio_core::{Error, Id, Result};
use writing_studio_diagnostics::SceneBeats;
use writing_studio_document::Document;
use writing_studio_format::{slug, SceneFile, SceneHeader};
use writing_studio_project::{BinderNode, BinderTree, NodeKind};

use crate::files::{bible_files, with_a_run_in_every_block, write_atomic, ProjectContent};
use crate::ProjectStore;

/// The heading each part of the bible gets in the export's `README.md`.
const BIBLE_SECTIONS: [(&str, &str); 6] = [
    ("characters", "Characters"),
    ("places", "Places"),
    ("cultures", "Cultures"),
    ("lore", "Lore"),
    ("timeline", "Timeline"),
    ("research", "Research"),
];

/// `01-`, `02-`… wide enough for every sibling, so the files sort in the
/// order of the book.
fn numbered(index: usize, count: usize, title: &str) -> String {
    let width = count.to_string().len().max(2);
    format!("{:0width$}-{}", index + 1, slug(title))
}

/// Text for a Markdown link, with the characters that would end it escaped.
fn link_text(title: &str) -> String {
    let title = if title.trim().is_empty() { "Untitled" } else { title };
    title
        .chars()
        .flat_map(|ch| match ch {
            '[' | ']' | '\\' | '*' | '_' | '`' | '<' => vec!['\\', ch],
            '\n' => vec![' '],
            _ => vec![ch],
        })
        .collect()
}

impl ProjectStore {
    /// Write the book into `target`, a folder that must be new or empty: the
    /// manuscript in reading order, one Markdown file for each scene, the
    /// bible one file for each entry, and a `README.md` that lists them all.
    /// The trash is left out. Returns how many Markdown files were written,
    /// not counting `README.md`.
    pub fn export_markdown(&self, target: &Path) -> Result<usize> {
        let not_empty = fs::read_dir(target).is_ok_and(|mut entries| entries.next().is_some());
        if not_empty {
            return Err(Error::message(format!(
                "{} is not empty; choose a new folder",
                target.display()
            )));
        }
        let content = ProjectContent::from_store(self)?;
        let mut contents = String::new();
        let mut written = 0;
        if let Some(root) = content.binder.draft_root() {
            let _ = writeln!(contents, "## Manuscript\n");
            written += export_children(&content, root.id, "manuscript", 0, target, &mut contents)?;
        }

        let (entries, _) = bible_files(
            &content.cast,
            &content.world,
            &content.library,
            &Default::default(),
            &mut Default::default(),
            &Default::default(),
        );
        for (folder, heading) in BIBLE_SECTIONS {
            let here: Vec<&SceneFile> = entries
                .iter()
                .filter(|(_, path, _)| path.split('/').next() == Some(folder))
                .map(|(_, _, file)| file)
                .collect();
            if here.is_empty() {
                continue;
            }
            let _ = writeln!(contents, "\n## {heading}\n");
            for (index, file) in here.iter().enumerate() {
                let path = format!("{folder}/{}.md", numbered(index, here.len(), &file.header.title));
                write_atomic(&target.join(&path), file.to_markdown().as_bytes())?;
                let _ = writeln!(contents, "- [{}]({path})", link_text(&file.header.title));
                written += 1;
            }
        }

        let readme = format!(
            "# {}\n\nExported from Writing Studio. Each scene and each entry of the bible is a Markdown file; this page lists them in order.\n\n{contents}",
            link_text(&content.meta.title)
        );
        write_atomic(&target.join("README.md"), readme.as_bytes())?;
        Ok(written)
    }

    /// Bring one Markdown file in as a new scene under `parent`. A header
    /// (`---` front matter) gives the scene its title, synopsis, status,
    /// label, color, goal, conflict, and outcome; with no title there, the
    /// scene is called `fallback_title`. The scene always gets a new id, so
    /// that bringing a file in twice cannot replace a scene.
    pub fn import_markdown_scene(
        &self,
        text: &str,
        fallback_title: &str,
        parent: Option<Id>,
        sort_index: i64,
    ) -> Result<(BinderNode, SceneBeats, Document)> {
        let file = SceneFile::parse_markdown(text).map_err(|err| Error::message(err.to_string()))?;
        let SceneHeader {
            title,
            synopsis,
            status,
            label,
            color,
            goal,
            conflict,
            outcome,
            ..
        } = file.header;
        let title = if title.trim().is_empty() { fallback_title.to_string() } else { title };
        let mut node = BinderNode::new(NodeKind::Scene, title, parent, sort_index);
        node.synopsis = synopsis;
        node.label = label;
        node.color = color;
        if !status.is_empty() {
            node.status = status;
        }
        let beats = SceneBeats { goal, conflict, outcome };
        let mut document = Document::new(node.id);
        document.blocks = with_a_run_in_every_block(file.blocks);

        if !beats.is_empty() {
            let mut all = self.load_scene_beats()?;
            all.insert(node.id, beats.clone());
            self.persist_scene_beats_cached(&all)?;
        }
        self.upsert_node(&node)?;
        self.save_document(&document)?;
        Ok((node, beats, document))
    }
}

/// Write the children of `parent` into `folder`, in order, and list them in
/// `contents` at `depth`. An item with text is a file; an item with children
/// is a folder beside it, with the same number and name.
fn export_children(
    content: &ProjectContent,
    parent: Id,
    folder: &str,
    depth: usize,
    target: &Path,
    contents: &mut String,
) -> Result<usize> {
    let tree: &BinderTree = &content.binder;
    let children: Vec<&BinderNode> = tree
        .children(Some(parent))
        .into_iter()
        .filter(|node| !node.kind.is_root() && !tree.is_in_trash(node.id))
        .collect();
    let mut written = 0;
    for (index, node) in children.iter().enumerate() {
        let name = numbered(index, children.len(), &node.title);
        let indent = "  ".repeat(depth);
        let has_children = tree
            .children(Some(node.id))
            .iter()
            .any(|child| !tree.is_in_trash(child.id));
        // A chapter that only holds scenes is a folder, not an empty file.
        let document = node
            .document_id
            .and_then(|id| content.documents.iter().find(|document| document.id == id))
            .filter(|document| !(has_children && document.to_plain_text().trim().is_empty()));
        match document {
            Some(document) => {
                let path = format!("{folder}/{name}.md");
                let beats = content.beats.get(&node.id).cloned().unwrap_or_default();
                let file = SceneFile {
                    header: SceneHeader {
                        id: None,
                        title: node.title.clone(),
                        synopsis: node.synopsis.clone(),
                        status: node.status.clone(),
                        label: node.label.clone(),
                        color: node.color.clone(),
                        goal: beats.goal,
                        conflict: beats.conflict,
                        outcome: beats.outcome,
                        other: Default::default(),
                    },
                    blocks: document.blocks.clone(),
                };
                write_atomic(&target.join(&path), file.to_markdown().as_bytes())?;
                let _ = writeln!(contents, "{indent}- [{}]({path})", link_text(&node.title));
                written += 1;
            }
            None => {
                let _ = writeln!(contents, "{indent}- {}", link_text(&node.title));
            }
        }
        written += export_children(content, node.id, &format!("{folder}/{name}"), depth + 1, target, contents)?;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_characters::Cast;
    use writing_studio_format::{canonical_blocks, same_content};

    fn temp(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ws-md-{tag}-{}", Id::new()))
    }

    /// Two books, a scene in the trash, and a character.
    fn sample(dir: &Path) -> ProjectStore {
        let store = ProjectStore::create(dir, "Tide").unwrap();
        let tree = store.load_binder().unwrap();
        let find = |kind| tree.nodes.iter().find(|node| node.kind == kind).unwrap().clone();
        let (root, chapter, scene, trash) = (
            find(NodeKind::DraftRoot),
            find(NodeKind::Chapter),
            find(NodeKind::Scene),
            find(NodeKind::TrashRoot),
        );
        let mut text = store.load_document(scene.id).unwrap();
        text.replace_html("<p>Mara ran for the <strong>north</strong> quay.</p>");
        store.save_document(&text).unwrap();
        let later = BinderNode::new(NodeKind::Scene, "The Quay", Some(chapter.id), 1);
        store.upsert_node(&later).unwrap();
        store.save_document(&Document::from_plain_text(later.id, "Second.")).unwrap();
        let mut cut = BinderNode::new(NodeKind::Scene, "Cut", Some(trash.id), 0);
        cut.deleted_at = Some(writing_studio_core::Timestamp::now());
        store.upsert_node(&cut).unwrap();
        store.save_document(&Document::from_plain_text(cut.id, "Gone.")).unwrap();
        let book = BinderNode::new(NodeKind::Book, "Inland [2]", Some(root.id), 5);
        store.upsert_node(&book).unwrap();
        let far = BinderNode::new(NodeKind::Scene, "Dust", Some(book.id), 0);
        store.upsert_node(&far).unwrap();
        store.save_document(&Document::from_plain_text(far.id, "Dust on the road.")).unwrap();
        let mut cast = Cast::new();
        let mara = cast.add_character();
        cast.set_name(mara, "Mara".into());
        store.persist_cast(&cast).unwrap();
        store
    }

    #[test]
    fn a_book_exports_in_reading_order_and_leaves_the_trash_out() {
        let (dir, target) = (temp("project"), temp("export"));
        let store = sample(&dir);
        assert_eq!(store.export_markdown(&target).unwrap(), 4);

        let scene = fs::read_to_string(target.join("manuscript/01-tide/01-chapter-1/01-scene-1.md")).unwrap();
        assert!(scene.starts_with("---\ntitle: Scene 1\n"), "{scene}");
        assert!(scene.ends_with("Mara ran for the **north** quay.\n"), "{scene}");
        assert!(!scene.contains("id:"));
        assert!(target.join("manuscript/01-tide/01-chapter-1/02-the-quay.md").exists());
        // A chapter that only holds scenes is a folder, not an empty file.
        assert!(!target.join("manuscript/01-tide/01-chapter-1.md").exists());
        assert!(target.join("manuscript/02-inland-2/01-dust.md").exists());
        assert!(target.join("characters/01-mara.md").exists());

        let readme = fs::read_to_string(target.join("README.md")).unwrap();
        assert!(readme.starts_with("# Tide\n"));
        let order: Vec<usize> = ["Scene 1", "The Quay", "Inland \\[2\\]", "## Characters"]
            .iter()
            .map(|item| readme.find(item).unwrap_or_else(|| panic!("{item} missing: {readme}")))
            .collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{readme}");
        assert!(!readme.contains("Cut") && !readme.contains("Story notes"));

        // A folder that is not empty is refused.
        assert!(store.export_markdown(&target).is_err());
        for folder in [dir, target] {
            let _ = fs::remove_dir_all(folder);
        }
    }

    #[test]
    fn exported_scenes_come_back_as_new_scenes() {
        let (dir, target) = (temp("source"), temp("files"));
        let store = sample(&dir);
        store.export_markdown(&target).unwrap();
        let chapter = store
            .load_binder()
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.kind == NodeKind::Chapter)
            .unwrap();
        let original = store
            .load_binder()
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.title == "Scene 1")
            .unwrap();

        let text = fs::read_to_string(target.join("manuscript/01-tide/01-chapter-1/01-scene-1.md")).unwrap();
        let (node, _, document) = store.import_markdown_scene(&text, "fallback", Some(chapter.id), 9).unwrap();
        assert_ne!(node.id, original.id);
        assert_eq!((node.title.as_str(), node.parent_id), ("Scene 1", Some(chapter.id)));
        let before = store.load_document(original.id).unwrap();
        assert!(same_content(&canonical_blocks(&document.blocks), &canonical_blocks(&before.blocks)));
        assert_eq!(store.load_document(node.id).unwrap().to_plain_text(), before.to_plain_text());

        // No header: the file's name is the title. A broken header is refused.
        let (plain, _, _) = store
            .import_markdown_scene("Just text, *with* a word.\n", "loose-notes", None, 10)
            .unwrap();
        assert_eq!(plain.title, "loose-notes");
        let error = store.import_markdown_scene("---\ntitle: x\n", "broken", None, 11).unwrap_err();
        assert!(error.to_string().contains("line"), "{error}");
        for folder in [dir, target] {
            let _ = fs::remove_dir_all(folder);
        }
    }
}
