//! Merging a scene with the next one, and splitting a scene at the caret.
//!
//! Each is one undo step. The open scene's text changes through its editor
//! session, so Ctrl+Z takes the text back as for any other edit; the step
//! kept here puts back the rest: the merged scene out of the trash, or the
//! new scene gone, and the tracking baseline as it was.

use writing_studio_core::{Id, Timestamp};
use writing_studio_document::{split_baseline, split_blocks, Block, Document};
use writing_studio_project::{BinderNode, NodeKind};

use super::StudioState;

#[derive(Clone)]
pub(super) struct StructureStep {
    /// The open scene's text right after the change, and right before it,
    /// as HTML. The step is undone only by the undo that goes from one to
    /// the other, so typing after a merge is undone first.
    after: String,
    before: String,
    /// The open scene's tracking baseline before the change, if it had one.
    baseline: Option<Document>,
    /// The open scene's item before the change, for its keywords and synopsis.
    node: BinderNode,
    kind: StepKind,
}

#[derive(Clone)]
enum StepKind {
    /// The scene that was joined in and trashed, and where it stood.
    Merged { scene: Id, parent: Id, after: Option<Id> },
    /// The scene the split made.
    Split { scene: Id },
}

impl StudioState {
    /// Whether the open scene has a scene after it in its book to merge.
    pub fn can_merge_with_next(&self) -> bool {
        self.open_scene().is_some_and(|id| self.binder.scene_after(id).is_some())
    }

    /// The open item, when it is a scene with its text in the editor.
    fn open_scene(&self) -> Option<Id> {
        let id = self.selected?;
        let node = self.binder.get(id)?;
        let open = self.session.as_ref().map(|session| session.document_id());
        (node.kind == NodeKind::Scene && node.document_id.is_some() && node.document_id == open).then_some(id)
    }

    /// Join the next scene in the book onto the end of the open one, and move
    /// the next scene to the trash. Notes, highlights, and tracked changes go
    /// with the text; keywords join the open scene's. Both scenes are kept as
    /// snapshots first.
    pub fn merge_with_next(&mut self) {
        self.flush_editor();
        let Some(first) = self.open_scene() else {
            self.status = "Open a scene to merge it with the next one.".into();
            return;
        };
        let Some(next) = self.binder.scene_after(first) else {
            self.status = "There is no scene after this one in the book.".into();
            return;
        };
        let (Some(store), Some(session)) = (self.store.clone(), self.session.as_ref()) else {
            return;
        };
        let (Some(first_node), Some(next_node)) = (self.binder.get(first).cloned(), self.binder.get(next).cloned())
        else {
            return;
        };
        let (Some(next_doc_id), Some(parent)) = (next_node.document_id, next_node.parent_id) else {
            return;
        };
        let current = session.document().clone();
        let first_baseline = self.tracking_for_session().map(|(tracking, _)| tracking.baseline.clone());
        let (next_doc, next_baseline) = {
            let store = store.borrow();
            let loaded = store.load_or_create_document(next_doc_id).and_then(|doc| {
                let baseline = store.load_tracking_baseline(next_doc_id)?;
                Ok((doc, baseline.map(|(baseline, _)| baseline)))
            });
            match loaded {
                Ok(found) => found,
                Err(err) => {
                    self.status = format!("Could not read “{}”: {err}", next_node.title);
                    return;
                }
            }
        };
        let kept = store
            .borrow()
            .snapshot_document(&current, "before merge")
            .and_then(|_| store.borrow().snapshot_document(&next_doc, "before merge"));
        if let Err(err) = kept {
            self.status = format!("Could not keep snapshots before merging, so nothing was merged: {err}");
            return;
        }

        let joined: Vec<Block> = if next_doc.to_plain_text().trim().is_empty() { Vec::new() } else { next_doc.blocks.clone() };
        // A baseline for the joined text when either scene was tracked, so
        // each part keeps its own changes and the untracked part has none.
        let merged_baseline = (first_baseline.is_some() || next_baseline.is_some()).then(|| {
            let mut baseline = first_baseline.clone().unwrap_or_else(|| current.clone());
            if !joined.is_empty() {
                let next_side = next_baseline.as_ref().map_or(&next_doc.blocks, |baseline| &baseline.blocks);
                baseline.blocks.extend(next_side.iter().cloned());
            }
            baseline.updated_at = Timestamp::now();
            baseline
        });
        if let Some(baseline) = &merged_baseline {
            if let Err(err) = store.borrow().save_tracking_baseline(baseline) {
                self.status = format!("Could not keep the tracked changes, so nothing was merged: {err}");
                return;
            }
        }

        let after = self.binder.previous_sibling(next);
        self.edit_text("", |doc| {
            doc.blocks.extend(joined);
            doc.updated_at = Timestamp::now();
            true
        });
        for word in &next_node.keywords {
            self.binder.add_keyword(first, word);
        }
        if let Some(node) = self.binder.get_mut(first) {
            if node.synopsis.trim().is_empty() {
                node.synopsis = next_node.synopsis.clone();
            }
        }
        if let Err(err) = self.binder.trash(next) {
            self.status = err.to_string();
            return;
        }
        if !self.persist_binder() {
            return;
        }
        self.flush_editor();
        self.load_tracking();
        self.structure_step = Some(StructureStep {
            after: self.open_html(),
            before: current.to_html(),
            baseline: first_baseline,
            node: first_node.clone(),
            kind: StepKind::Merged { scene: next, parent, after },
        });
        self.status = format!(
            "Merged “{}” into “{}”. It is in the trash; Ctrl+Z puts it back.",
            next_node.title, first_node.title
        );
    }

    /// Make a new scene, right after the open one, from the text `at`
    /// characters in to the end. The new scene takes the open one's status,
    /// keywords, and color, and its tracked changes for that text. The open
    /// scene is kept as a snapshot first.
    pub fn split_at(&mut self, at: usize) {
        self.flush_editor();
        let Some(id) = self.open_scene() else {
            self.status = "Open a scene to split it.".into();
            return;
        };
        let (Some(store), Some(session)) = (self.store.clone(), self.session.as_ref()) else {
            return;
        };
        let current = session.document().clone();
        let Some(point) = current.split_point(at) else {
            self.status = "Put the caret where the new scene should start, with words before and after it.".into();
            return;
        };
        let Some(node) = self.binder.get(id).cloned() else {
            return;
        };
        let Some(parent) = node.parent_id else {
            return;
        };
        let baseline = self.tracking_for_session().map(|(tracking, _)| tracking.baseline.clone());
        if let Err(err) = store.borrow().snapshot_document(&current, "before split") {
            self.status = format!("Could not keep a snapshot before splitting, so nothing was split: {err}");
            return;
        }

        let (head, tail) = split_blocks(&current.blocks, point);
        let mut scene = BinderNode::new(NodeKind::Scene, format!("{} (continued)", node.title), Some(parent), node.sort_index + 1);
        scene.status = node.status.clone();
        scene.keywords = node.keywords.clone();
        scene.color = node.color.clone();
        let (scene_id, Some(doc_id)) = (scene.id, scene.document_id) else {
            return;
        };
        let mut text = Document::new(doc_id);
        text.blocks = tail;
        // The new scene is written before the open one is cut, so a failure
        // leaves the text whole.
        let written = {
            let store = store.borrow();
            store.upsert_node(&scene).and_then(|()| store.save_document(&text)).and_then(|()| {
                let Some(baseline) = &baseline else {
                    return Ok(());
                };
                let (head_base, tail_base) = split_baseline(baseline, &current, point);
                let part = |id: Id, blocks: Vec<Block>| Document {
                    id,
                    blocks: if blocks.is_empty() { vec![Block::paragraph("")] } else { blocks },
                    updated_at: Timestamp::now(),
                    ..baseline.clone()
                };
                store.save_tracking_baseline(&part(doc_id, tail_base))?;
                store.save_tracking_baseline(&part(current.id, head_base))
            })
        };
        if let Err(err) = written {
            self.status = format!("Could not split the scene: {err}");
            return;
        }
        let _ = self.manuscript_index.borrow_mut().upsert(doc_id, &text.to_plain_text());
        self.binder.nodes.push(scene);
        if let Err(err) = self.binder.put_after(scene_id, parent, Some(id)) {
            self.status = err.to_string();
            return;
        }
        if !self.persist_binder() {
            return;
        }
        self.edit_text("", |doc| {
            doc.blocks = head;
            doc.updated_at = Timestamp::now();
            true
        });
        self.flush_editor();
        self.load_tracking();
        self.structure_step = Some(StructureStep {
            after: self.open_html(),
            before: current.to_html(),
            baseline,
            node: node.clone(),
            kind: StepKind::Split { scene: scene_id },
        });
        self.status = format!(
            "Split “{}”. The text after the caret is the new scene “{} (continued)”; Ctrl+Z joins them again.",
            node.title, node.title
        );
    }

    fn open_html(&self) -> String {
        self.session.as_ref().map(|session| session.document().to_html()).unwrap_or_default()
    }

    /// Whether the next undo is the one that undoes the last merge or split.
    pub(super) fn undo_is_structure_step(&self) -> bool {
        let (Some(step), Some(session)) = (&self.structure_step, &self.session) else {
            return false;
        };
        session.undo_target().is_some_and(|target| target.to_html() == step.before)
            && session.document().to_html() == step.after
    }

    /// Put back what a merge or a split changed besides the text, once undo
    /// has put the text back.
    pub(super) fn undo_structure_step(&mut self) {
        let Some(step) = self.structure_step.take() else {
            return;
        };
        let Some(store) = self.store.clone() else {
            return;
        };
        if let Some(session) = &mut self.session {
            session.clear_redo();
        }
        let open = self.session.as_ref().map(|session| session.document_id());
        let tracked = match (&step.baseline, open) {
            (Some(baseline), _) => store.borrow().save_tracking_baseline(baseline),
            (None, Some(open)) => store.borrow().stop_tracking(open),
            (None, None) => Ok(()),
        };
        if let Err(err) = tracked {
            self.status = format!("Could not put the tracked changes back: {err}");
        }
        if let Some(node) = self.binder.get_mut(step.node.id) {
            node.keywords = step.node.keywords.clone();
            node.synopsis = step.node.synopsis.clone();
        }
        let status = match step.kind {
            StepKind::Merged { scene, parent, after } => {
                let title = self.binder.get(scene).map(|node| node.title.clone()).unwrap_or_default();
                match self.binder.put_after(scene, parent, after) {
                    Ok(()) => format!("Undid the merge. “{title}” is back in its place."),
                    Err(_) => format!("Undid the merge. “{title}” stays in the trash."),
                }
            }
            StepKind::Split { scene } => {
                let doc_id = self.binder.get(scene).and_then(|node| node.document_id);
                let removed = {
                    let store = store.borrow();
                    doc_id
                        .map_or(Ok(()), |doc_id| store.delete_document(doc_id))
                        .and_then(|()| store.delete_node(scene))
                };
                match removed {
                    Ok(()) => {
                        self.binder.nodes.retain(|node| node.id != scene);
                        self.beats.remove(&scene);
                        if let Some(doc_id) = doc_id {
                            let _ = self.manuscript_index.borrow_mut().remove(doc_id);
                        }
                        "Undid the split. The scene is whole again.".to_string()
                    }
                    Err(err) => format!("Undid the split, but the new scene could not be removed: {err}"),
                }
            }
        };
        let saved = self.persist_binder();
        self.flush_editor();
        self.load_tracking();
        if saved {
            self.status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn open_state(tag: &str) -> (StudioState, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ws-ui-{tag}-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut state = StudioState::default();
        state.create_project_in(dir.clone());
        assert!(state.session.is_some(), "{}", state.status);
        (state, dir)
    }

    fn text_of(state: &StudioState, id: Id) -> String {
        let doc_id = state.binder.get(id).and_then(|node| node.document_id).unwrap();
        state.store.as_ref().unwrap().borrow().load_document(doc_id).unwrap().to_plain_text()
    }

    #[test]
    fn a_split_makes_the_next_scene_and_undo_joins_them_again() {
        let (mut state, dir) = open_state("split");
        let first = state.selected.unwrap();
        state.apply_editor_html(
            r#"<p>Before the cut.</p><p>After <span class="ws-comment" data-comment="check">the cut</span>.</p>"#.into(),
        );
        state.binder.add_keyword(first, "Mara");
        state.start_tracking();
        state.apply_editor_text("Before the cut.\nAfter the change.".into());
        let tracked_before = state.tracked_changes().len();
        assert_eq!(tracked_before, 1);

        // "Before the cut." is 15 characters.
        state.split_at(15);
        let parent = state.binder.get(first).unwrap().parent_id;
        let order: Vec<Id> = state.binder.children(parent).iter().map(|node| node.id).collect();
        let at = order.iter().position(|id| *id == first).unwrap();
        let second = order[at + 1];
        let new = state.binder.get(second).unwrap().clone();
        assert!(new.title.ends_with("(continued)"));
        assert_eq!(new.keywords, ["Mara"]);
        assert_eq!(text_of(&state, first), "Before the cut.");
        assert_eq!(text_of(&state, second), "After the change.");
        assert!(state.tracked_changes().is_empty(), "the change went with its text");
        assert!(!state.snapshots_for_selected().is_empty(), "a snapshot was taken first");
        let new_doc = new.document_id.unwrap();
        let (baseline, _) = state.store.as_ref().unwrap().borrow().load_tracking_baseline(new_doc).unwrap().unwrap();
        assert_eq!(baseline.to_plain_text(), "After the cut.", "the new scene keeps the change as tracked");

        state.undo();
        assert_eq!(state.editor_text, "Before the cut.\nAfter the change.");
        assert!(state.binder.get(second).is_none(), "the new scene is gone");
        assert_eq!(state.tracked_changes().len(), 1, "the baseline is back");
        assert!(state.status.starts_with("Undid the split"), "{}", state.status);
        state.redo();
        assert_eq!(state.editor_text, "Before the cut.\nAfter the change.", "redo does not cut it again");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_merge_joins_the_next_scene_and_undo_puts_it_back() {
        let (mut state, dir) = open_state("merge");
        let first = state.selected.unwrap();
        state.apply_editor_text("First part.".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_html(r#"<p><mark data-highlight="yellow">Second</mark> part.</p>"#.into());
        state.binder.add_keyword(second, "Flashback");
        state.start_tracking();
        state.apply_editor_text("Second part, changed.".into());
        state.select_node(first);
        assert_eq!(state.binder.scene_after(first), Some(second));
        assert!(state.can_merge_with_next());

        state.merge_with_next();
        assert_eq!(state.editor_text, "First part.\nSecond part, changed.");
        assert!(state.binder.is_in_trash(second));
        assert_eq!(state.binder.get(first).unwrap().keywords, ["Flashback"]);
        assert_eq!(state.tracked_changes().len(), 1, "the second scene's change came with it");
        assert!(state.status.starts_with("Merged"), "{}", state.status);
        assert_eq!(state.snapshots_for_selected().len(), 1);

        state.undo();
        assert_eq!(state.editor_text, "First part.");
        assert!(!state.binder.is_in_trash(second));
        assert_eq!(state.binder.scene_after(first), Some(second), "back in its place");
        assert!(state.binder.get(first).unwrap().keywords.is_empty());
        assert!(!state.is_tracking(), "the first scene was not tracked before");
        assert_eq!(text_of(&state, first), "First part.");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn typing_after_a_merge_is_undone_before_the_merge() {
        let (mut state, dir) = open_state("merge-typing");
        let first = state.selected.unwrap();
        state.apply_editor_text("One.".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_text("Two.".into());
        state.select_node(first);
        state.merge_with_next();
        state.apply_editor_text("One.\nTwo.\nThree.".into());
        state.undo();
        assert_eq!(state.editor_text, "One.\nTwo.");
        assert!(state.binder.is_in_trash(second), "only the typing was undone");
        state.undo();
        assert_eq!(state.editor_text, "One.");
        assert!(!state.binder.is_in_trash(second));
        let _ = std::fs::remove_dir_all(dir);
    }
}
