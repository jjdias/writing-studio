//! Merging a scene with the next one, and splitting a scene at the caret.
//!
//! Each is one undo step. The open scene's text changes through its editor
//! session, so Ctrl+Z takes the text back as for any other edit; the step
//! kept here puts back the rest: the merged scene out of the trash, or the
//! new scene gone, and the tracking baseline as it was.

use writing_studio_core::{Id, Timestamp};
use writing_studio_document::{split_baseline, split_blocks, Block, Document};
use writing_studio_diagnostics::SceneBeats;
use writing_studio_project::{BinderNode, NodeKind};

use super::{StudioState, UndoOffer};

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
    /// The scenes that were joined in and trashed, in reading order.
    Merged(Vec<MergedScene>),
    /// The scene the split made.
    Split { scene: Id },
}

/// A scene a merge joined in, where it stood, and what it brought over.
#[derive(Clone)]
struct MergedScene {
    scene: Id,
    parent: Id,
    after: Option<Id>,
    carried: Carried,
}

/// A scene about to be joined in, read before anything changes.
struct Joined {
    node: BinderNode,
    parent: Id,
    after: Option<Id>,
    doc: Document,
    baseline: Option<Document>,
}

/// What a merge brought over from the joined scene, so undo takes back
/// exactly that.
#[derive(Clone, Default)]
struct Carried {
    /// Plot points that pointed at the joined scene and now point at the open one.
    points: Vec<Id>,
    /// Characters, and places, cultures, lore, and events, that were linked
    /// to the joined scene and were given a link to the open one. The joined
    /// scene keeps its own links, so restoring it from the trash finds them.
    characters: Vec<Id>,
    world: Vec<Id>,
    /// The open scene's goal, conflict, and outcome before the joined scene's
    /// were added, when they changed.
    beats: Option<Option<SceneBeats>>,
}

/// Two beats as one: each written one, joined by a line break.
fn join_beat(first: &str, second: &str) -> String {
    match (first.trim().is_empty(), second.trim().is_empty()) {
        (_, true) => first.to_string(),
        (true, false) => second.to_string(),
        (false, false) => format!("{first}\n{second}"),
    }
}

/// The title of the scene a split makes: the open scene's title numbered,
/// "Tide (2)", and the next free number when the chapter already has one,
/// so splitting "Tide (2)" gives "Tide (3)". A number in brackets at the end,
/// or the "(continued)" older splits added, is not part of the name.
fn split_title(title: &str, siblings: &[String]) -> String {
    let base = split_base(title).0;
    let taken = siblings
        .iter()
        .filter_map(|sibling| match split_base(sibling) {
            (name, number) if name == base => Some(number.unwrap_or(1)),
            _ => None,
        })
        .max()
        .unwrap_or(1);
    if base.is_empty() {
        return format!("Scene ({})", taken + 1);
    }
    format!("{base} ({})", taken + 1)
}

/// A title without a trailing " (n)" or " (continued)" (any number of
/// them), and the number it had. A number of 1000 or more, such as a year
/// in "Summer (1999)", is part of the name.
fn split_base(title: &str) -> (&str, Option<u32>) {
    let mut rest = title.trim();
    let mut number = None;
    while let Some(open) = rest.strip_suffix(')').and_then(|inner| inner.rfind(" (").map(|at| (inner, at))) {
        let (inner, at) = open;
        let tag = &inner[at + 2..];
        if tag == "continued" {
            number = Some(number.unwrap_or(1) + 1);
        } else if let Some(n) = tag.parse::<u32>().ok().filter(|n| (1..1000).contains(n)) {
            number = Some(number.map_or(n, |found: u32| found.max(n)));
        } else {
            break;
        }
        rest = inner[..at].trim_end();
    }
    (rest, number)
}

impl StudioState {
    /// Give the open scene what pointed at the joined one: plot points move
    /// to it, and characters and world entries linked to the joined scene are
    /// linked to it too. The goal, conflict, and outcome are joined.
    fn carry_links(&mut self, first: Id, next: Id) -> Carried {
        let mut carried = Carried::default();
        let points: Vec<Id> =
            self.plot.points.iter().filter(|point| point.scene_id == Some(next)).map(|point| point.id).collect();
        for point in points {
            if self.plot.set_point_scene(point, Some(first)) {
                carried.points.push(point);
            }
        }
        let characters: Vec<Id> = self
            .cast
            .characters
            .iter()
            .filter(|character| character.linked_ids.contains(&next))
            .map(|character| character.id)
            .collect();
        for character in characters {
            if self.cast.link_scene(character, first) {
                carried.characters.push(character);
            }
        }
        let world = &self.world;
        let linked = |scenes: &Vec<Id>| scenes.contains(&next);
        let entries: Vec<(u8, Id)> = world
            .locations
            .iter()
            .filter(|entry| linked(&entry.linked_ids))
            .map(|entry| (0, entry.id))
            .chain(world.cultures.iter().filter(|entry| linked(&entry.linked_ids)).map(|entry| (1, entry.id)))
            .chain(world.lore.iter().filter(|entry| linked(&entry.linked_ids)).map(|entry| (2, entry.id)))
            .chain(world.events.iter().filter(|entry| linked(&entry.linked_ids)).map(|entry| (3, entry.id)))
            .collect();
        for (kind, id) in entries {
            let added = match kind {
                0 => self.world.link_location_scene(id, first),
                1 => self.world.link_culture_scene(id, first),
                2 => self.world.link_lore_scene(id, first),
                _ => self.world.link_event_scene(id, first),
            };
            if added {
                carried.world.push(id);
            }
        }
        if let Some(joined) = self.beats.get(&next).filter(|beats| !beats.is_empty()).cloned() {
            let before = self.beats.get(&first).cloned();
            let mine = before.clone().unwrap_or_default();
            self.beats.insert(
                first,
                SceneBeats {
                    goal: join_beat(&mine.goal, &joined.goal),
                    conflict: join_beat(&mine.conflict, &joined.conflict),
                    outcome: join_beat(&mine.outcome, &joined.outcome),
                },
            );
            carried.beats = Some(before);
        }
        self.persist_carried(&carried);
        carried
    }

    /// Take back what [`Self::carry_links`] gave the open scene.
    fn uncarry_links(&mut self, first: Id, next: Id, carried: Carried) {
        for point in &carried.points {
            if self.plot.points.iter().any(|kept| kept.id == *point && kept.scene_id == Some(first)) {
                self.plot.set_point_scene(*point, Some(next));
            }
        }
        for character in &carried.characters {
            self.cast.unlink_scene(*character, first);
        }
        for id in &carried.world {
            // The id is one kind of entry; the other three find nothing.
            self.world.unlink_location_scene(*id, first);
            self.world.unlink_culture_scene(*id, first);
            self.world.unlink_lore_scene(*id, first);
            self.world.unlink_event_scene(*id, first);
        }
        if let Some(before) = &carried.beats {
            match before {
                Some(beats) => self.beats.insert(first, beats.clone()),
                None => self.beats.remove(&first),
            };
        }
        self.persist_carried(&carried);
    }

    fn persist_carried(&mut self, carried: &Carried) {
        if !carried.points.is_empty() && !self.persist_plot_now() {
            self.plot_dirty = true;
        }
        if !carried.characters.is_empty() && !self.persist_cast_now() {
            self.cast_dirty = true;
        }
        if !carried.world.is_empty() && !self.persist_world_now() {
            self.world_dirty = true;
        }
        if carried.beats.is_some() && !self.persist_beats_now() {
            self.beats_dirty = true;
        }
    }

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
    /// the next scene to the trash.
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
        self.merge_into_open(vec![next]);
    }

    /// Join the picked scenes into the first of them in reading order, and
    /// open it.
    pub fn merge_picked(&mut self) {
        let order = self.reading_order();
        let mut scenes: Vec<Id> = order.into_iter().filter(|id| self.picked.contains(id)).collect();
        if scenes.len() < 2 {
            self.status = "Pick two or more scenes to merge them.".into();
            return;
        }
        let first = scenes.remove(0);
        self.item_menu = None;
        self.select_node(first);
        self.mode = super::WorkspaceMode::Writing;
        self.picked.clear();
        self.merge_into_open(scenes);
    }

    /// Ctrl+click on a scene card: add it to the picked scenes, or take it
    /// out. The open scene is picked with it, so the first Ctrl+click picks
    /// two.
    pub fn toggle_pick(&mut self, id: Id) {
        self.item_menu = None;
        if self.picked.is_empty() {
            if let Some(open) = self.selected.filter(|open| *open != id && self.is_scene(*open)) {
                self.picked.push(open);
            }
        }
        if let Some(at) = self.picked.iter().position(|kept| *kept == id) {
            self.picked.remove(at);
        } else if self.is_scene(id) {
            self.picked.push(id);
        }
        self.note_picked();
    }

    /// Shift+click on a scene card: pick every scene from the open one, or
    /// the last picked, to this one, in reading order.
    pub fn pick_range(&mut self, id: Id) {
        self.item_menu = None;
        let order = self.reading_order();
        let anchor = self.picked.last().copied().or(self.selected).filter(|anchor| order.contains(anchor));
        let (Some(anchor), Some(to)) = (anchor, order.iter().position(|scene| *scene == id)) else {
            self.toggle_pick(id);
            return;
        };
        let from = order.iter().position(|scene| *scene == anchor).unwrap_or(to);
        let (low, high) = if from <= to { (from, to) } else { (to, from) };
        for scene in &order[low..=high] {
            if !self.picked.contains(scene) {
                self.picked.push(*scene);
            }
        }
        self.note_picked();
    }

    pub fn clear_picked(&mut self) -> bool {
        !std::mem::take(&mut self.picked).is_empty()
    }

    fn is_scene(&self, id: Id) -> bool {
        self.binder.get(id).is_some_and(|node| node.kind == NodeKind::Scene) && !self.binder.is_in_trash(id)
    }

    fn note_picked(&mut self) {
        if self.picked.len() > 1 {
            self.status = format!(
                "{} scenes picked. Right-click one of them to act on all; Escape lets them go.",
                self.picked.len()
            );
        }
    }

    /// The picked scenes when `id` is one of them and there are several, for
    /// a menu that acts on all of them.
    pub fn picked_with(&self, id: Id) -> Vec<Id> {
        if self.picked.len() > 1 && self.picked.contains(&id) {
            self.reading_order().into_iter().filter(|scene| self.picked.contains(scene)).collect()
        } else {
            Vec::new()
        }
    }

    /// Give every picked scene a status.
    pub fn set_picked_status(&mut self, status: String) {
        self.item_menu = None;
        for id in self.picked.clone() {
            self.update_node_status(id, status.clone());
        }
        self.save_all();
        self.status = format!("{} scenes are now {}.", self.picked.len(), status);
    }

    /// Move every picked scene to the trash.
    pub fn trash_picked(&mut self) {
        self.item_menu = None;
        let picked = std::mem::take(&mut self.picked);
        for id in &picked {
            self.trash_item(*id);
        }
        self.status = format!("Moved {} scenes to the trash. Restore them from Trash in the library.", picked.len());
    }

    /// Every scene in the manuscript, in reading order.
    pub(super) fn reading_order(&self) -> Vec<Id> {
        let Some(draft) = self.binder.draft_root() else {
            return Vec::new();
        };
        self.binder
            .scenes_under(draft.id)
            .into_iter()
            .map(|(_, scene)| scene)
            .filter(|scene| self.binder.get(*scene).is_some_and(|node| node.kind == NodeKind::Scene))
            .collect()
    }

    /// Join `others`, in the order given, onto the end of the open scene and
    /// move them to the trash. Notes, highlights, and tracked changes go with
    /// the text; keywords join the open scene's, and its synopsis is the
    /// first one written. Every scene is kept as a snapshot first.
    fn merge_into_open(&mut self, others: Vec<Id>) {
        let Some(first) = self.open_scene() else {
            self.status = "Open a scene to merge into it.".into();
            return;
        };
        let others: Vec<Id> = others.into_iter().filter(|id| *id != first).collect();
        let (Some(store), Some(session)) = (self.store.clone(), self.session.as_ref()) else {
            return;
        };
        let Some(first_node) = self.binder.get(first).cloned() else {
            return;
        };
        let current = session.document().clone();
        let first_baseline = self.tracking_for_session().map(|(tracking, _)| tracking.baseline.clone());
        // Each joined scene with its text, its baseline, and where it stood.
        let mut joined = Vec::new();
        for id in &others {
            let Some(node) = self.binder.get(*id).cloned() else {
                return;
            };
            let (Some(doc_id), Some(parent)) = (node.document_id, node.parent_id) else {
                return;
            };
            let loaded = {
                let store = store.borrow();
                store.load_or_create_document(doc_id).and_then(|doc| {
                    let baseline = store.load_tracking_baseline(doc_id)?;
                    Ok((doc, baseline.map(|(baseline, _)| baseline)))
                })
            };
            match loaded {
                Ok((doc, baseline)) => {
                    let after = self.binder.previous_sibling(*id);
                    joined.push(Joined { node, parent, after, doc, baseline });
                }
                Err(err) => {
                    self.status = format!("Could not read “{}”: {err}", node.title);
                    return;
                }
            }
        }
        if joined.is_empty() {
            return;
        }
        let kept = std::iter::once(&current)
            .chain(joined.iter().map(|scene| &scene.doc))
            .try_for_each(|doc| store.borrow().snapshot_document(doc, "before merge").map(|_| ()));
        if let Err(err) = kept {
            self.status = format!("Could not keep snapshots before merging, so nothing was merged: {err}");
            return;
        }

        // A scene with no words adds nothing but goes to the trash all the same.
        let has_words = |doc: &Document| !doc.to_plain_text().trim().is_empty();
        // A baseline for the joined text when any scene was tracked, so each
        // part keeps its own changes and the untracked parts have none.
        if first_baseline.is_some() || joined.iter().any(|scene| scene.baseline.is_some()) {
            let mut baseline = first_baseline.clone().unwrap_or_else(|| current.clone());
            for scene in &joined {
                if has_words(&scene.doc) {
                    let side = scene.baseline.as_ref().map_or(&scene.doc.blocks, |own| &own.blocks);
                    baseline.blocks.extend(side.iter().cloned());
                }
            }
            baseline.updated_at = Timestamp::now();
            if let Err(err) = store.borrow().save_tracking_baseline(&baseline) {
                self.status = format!("Could not keep the tracked changes, so nothing was merged: {err}");
                return;
            }
        }

        let blocks: Vec<Block> = joined
            .iter()
            .filter(|scene| has_words(&scene.doc))
            .flat_map(|scene| scene.doc.blocks.iter().cloned())
            .collect();
        self.edit_text("", |doc| {
            doc.blocks.extend(blocks);
            doc.updated_at = Timestamp::now();
            true
        });
        for scene in &joined {
            for word in &scene.node.keywords {
                self.binder.carry_keyword(first, word);
            }
            if let Some(open) = self.binder.get_mut(first) {
                if open.synopsis.trim().is_empty() {
                    open.synopsis = scene.node.synopsis.clone();
                }
            }
            if let Err(err) = self.binder.trash(scene.node.id) {
                self.status = err.to_string();
                return;
            }
        }
        if !self.persist_binder() {
            return;
        }
        let merged: Vec<MergedScene> = joined
            .iter()
            .map(|scene| MergedScene {
                scene: scene.node.id,
                parent: scene.parent,
                after: scene.after,
                carried: self.carry_links(first, scene.node.id),
            })
            .collect();
        self.flush_editor();
        self.load_tracking();
        self.structure_step = Some(StructureStep {
            after: self.open_html(),
            before: current.to_html(),
            baseline: first_baseline,
            node: first_node.clone(),
            kind: StepKind::Merged(merged),
        });
        self.status = match &joined[..] {
            [scene] => format!(
                "Merged “{}” into “{}”. It is in the trash; Ctrl+Z puts it back.",
                scene.node.title, first_node.title
            ),
            many => format!(
                "Merged {} scenes into “{}”. They are in the trash; Ctrl+Z puts them back.",
                many.len(),
                first_node.title
            ),
        };
        self.offer_undo(UndoOffer::Structure);
    }

    /// Make a new scene, right after the open one, from the text `at`
    /// characters in to the end. The new scene takes the open one's status,
    /// keywords, and color, and its tracked changes for that text. The open
    /// scene is kept as a snapshot first.
    /// Split where the caret is on the Markdown page, given the page's text
    /// before the caret: read as Markdown, that is where the cut goes, in
    /// characters as `split_at` counts them.
    pub fn split_at_markdown(&mut self, before: &str) {
        let read = writing_studio_document::shown_length_of(&writing_studio_format::markdown_to_blocks(before));
        // Reading drops the spaces at the end, which are text in the scene:
        // the cut goes after them, as on the rich-text page.
        let spaces = before.chars().rev().take_while(|ch| *ch == ' ' || *ch == '\t').count();
        self.split_at(read + spaces);
    }

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
        let siblings: Vec<String> = self.binder.children(Some(parent)).iter().map(|sibling| sibling.title.clone()).collect();
        let title = split_title(&node.title, &siblings);
        let mut scene = BinderNode::new(NodeKind::Scene, title.clone(), Some(parent), node.sort_index + 1);
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
            "Split “{}”. The text after the caret is the new scene “{title}”; Ctrl+Z joins them again.",
            node.title
        );
        self.offer_undo(UndoOffer::Structure);
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
            StepKind::Merged(merged) => {
                // Links come back last one first, so the open scene's beats
                // end as they were before the first was added.
                for scene in merged.iter().rev() {
                    self.uncarry_links(step.node.id, scene.scene, scene.carried.clone());
                }
                // Scenes go back first one first: each was after the one before.
                let mut kept = Vec::new();
                for scene in &merged {
                    if self.binder.put_after(scene.scene, scene.parent, scene.after).is_err() {
                        kept.push(self.binder.get(scene.scene).map(|node| node.title.clone()).unwrap_or_default());
                    }
                }
                match (merged.len(), kept.is_empty()) {
                    (1, true) => {
                        let title = self.binder.get(merged[0].scene).map(|node| node.title.clone()).unwrap_or_default();
                        format!("Undid the merge. “{title}” is back in its place.")
                    }
                    (_, true) => "Undid the merge. The scenes are back in their places.".to_string(),
                    (_, false) => format!("Undid the merge. “{}” stays in the trash.", kept.join("”, “")),
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

    #[test]
    fn a_split_numbers_the_new_scene_after_the_highest_number_in_the_chapter() {
        let titles = |list: &[&str]| list.iter().map(|title| title.to_string()).collect::<Vec<_>>();
        assert_eq!(split_title("Tide", &titles(&["Tide"])), "Tide (2)");
        assert_eq!(split_title("Tide (2)", &titles(&["Tide", "Tide (2)"])), "Tide (3)");
        assert_eq!(split_title("Tide", &titles(&["Tide", "Tide (2)", "Tide (3)"])), "Tide (4)", "no number twice");
        assert_eq!(split_title("Tide (continued) (continued)", &titles(&["Tide (continued) (continued)"])), "Tide (4)");
        assert_eq!(split_title("Summer (1999)", &titles(&["Summer (1999)"])), "Summer (1999) (2)", "a year is a name");
        assert_eq!(split_title("Act (two)", &titles(&["Act (two)"])), "Act (two) (2)");
        assert_eq!(split_title("  ", &titles(&["  "])), "Scene (2)");
    }

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
        state.apply_editor_markdown("Before the cut.\n\nAfter {==the cut==}{>>check<<}.");
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
        assert_eq!(new.title, format!("{} (2)", state.binder.get(first).unwrap().title));
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
    fn a_split_on_the_markdown_page_cuts_where_the_caret_is_in_the_text() {
        let (mut state, dir) = open_state("split-markdown");
        let first = state.selected.unwrap();
        let page = "# One\n\nShe opened the **red** door. A gull cried.\n\nNobody spoke.";
        state.apply_editor_markdown(page);
        // The caret just before "A gull": the marks before it are not text.
        let before = &page[..page.find("A gull").unwrap()];
        state.split_at_markdown(before);
        let parent = state.binder.get(first).unwrap().parent_id;
        let order: Vec<Id> = state.binder.children(parent).iter().map(|node| node.id).collect();
        let second = order[order.iter().position(|id| *id == first).unwrap() + 1];
        assert_eq!(text_of(&state, first), "One\nShe opened the red door. ");
        assert_eq!(text_of(&state, second), "A gull cried.\nNobody spoke.");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_merge_joins_the_next_scene_and_undo_puts_it_back() {
        let (mut state, dir) = open_state("merge");
        let first = state.selected.unwrap();
        state.apply_editor_text("First part.".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_markdown("{==Second==} part.");
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
    fn a_merge_brings_the_joined_scenes_links_and_beats_and_undo_takes_them_back() {
        let (mut state, dir) = open_state("merge-links");
        let first = state.selected.unwrap();
        state.apply_editor_text("One.".into());
        state.beats.insert(first, SceneBeats { goal: "Find the key".into(), ..SceneBeats::default() });
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_text("Two.".into());
        state.beats.insert(second, SceneBeats { goal: "Open the door".into(), outcome: "Locked".into(), ..SceneBeats::default() });
        let line = state.plot.add_line();
        let point = state.plot.add_point(line).unwrap();
        state.plot.set_point_scene(point, Some(second));
        let hero = state.cast.add_character();
        state.cast.link_scene(hero, second);
        let place = state.world.add_location();
        state.world.link_location_scene(place, second);
        state.select_node(first);

        state.merge_with_next();
        let point_scene = |state: &StudioState| state.plot.points.iter().find(|kept| kept.id == point).unwrap().scene_id;
        assert_eq!(point_scene(&state), Some(first));
        let hero_scenes = |state: &StudioState| state.cast.characters.iter().find(|kept| kept.id == hero).unwrap().linked_ids.clone();
        assert_eq!(hero_scenes(&state), [second, first], "the trashed scene keeps its link for restoring");
        assert!(state.world.locations.iter().find(|kept| kept.id == place).unwrap().linked_ids.contains(&first));
        let beats = state.beats.get(&first).unwrap();
        assert_eq!(beats.goal, "Find the key
Open the door");
        assert_eq!(beats.outcome, "Locked");

        state.undo();
        assert_eq!(point_scene(&state), Some(second));
        assert_eq!(hero_scenes(&state), [second]);
        assert!(!state.world.locations.iter().find(|kept| kept.id == place).unwrap().linked_ids.contains(&first));
        assert_eq!(state.beats.get(&first).unwrap().goal, "Find the key");
        assert!(state.beats.get(&first).unwrap().outcome.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_right_click_menu_opens_on_items_and_acts_on_the_one_clicked() {
        let (mut state, dir) = open_state("item-menu");
        let first = state.selected.unwrap();
        state.apply_editor_text("One.".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_text("Two.".into());
        let root = state.binder.draft_root().unwrap().id;
        state.open_item_menu(root, 10.0, 10.0);
        assert!(state.item_menu.is_none(), "a root has no menu");
        state.open_item_menu(first, 10.0, 20.0);
        assert_eq!(state.item_menu.map(|menu| menu.id), Some(first));
        assert!(state.close_popup(), "Escape closes it");
        assert!(state.item_menu.is_none());

        state.set_item_status(second, "Revise".into());
        assert_eq!(state.binder.get(second).unwrap().status, "Revise");
        state.merge_item_with_next(first);
        assert_eq!(state.selected, Some(first), "the clicked scene is opened first");
        assert!(state.binder.is_in_trash(second));
        state.restore_item(second);
        assert!(!state.binder.is_in_trash(second));
        state.start_rename(root);
        assert_eq!(state.renaming, None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn go_to_finds_names_with_every_word_and_opens_them() {
        let (mut state, dir) = open_state("go-to");
        let first = state.selected.unwrap();
        state.update_node_title(first, "Harbor at dawn".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.update_node_title(second, "The dawn patrol".into());
        let chapter = state.binder.get(first).unwrap().parent_id.unwrap();
        let chapter_name = state.binder.get(chapter).unwrap().title.clone();

        let found: Vec<Id> = state.quick_matches("dawn").iter().map(|item| item.id).collect();
        assert_eq!(found, [first, second]);
        let found: Vec<Id> = state.quick_matches("the dawn").iter().map(|item| item.id).collect();
        assert_eq!(found, [second], "every word is needed");
        let found = state.quick_matches(&format!("{} harbor", chapter_name.to_lowercase()));
        assert_eq!(found.len(), 1, "the chapter's name counts too");
        assert_eq!(found[0].place.contains(&chapter_name), true);
        let starts: Vec<Id> = state.quick_matches("the").iter().map(|item| item.id).collect();
        assert_eq!(starts.first(), Some(&second), "a name that starts with it comes first");

        state.toggle_quick_open();
        assert!(state.quick_open);
        state.go_to(first);
        assert!(!state.quick_open);
        assert_eq!(state.selected, Some(first));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_status_lines_undo_takes_back_a_trash_or_a_merge() {
        let (mut state, dir) = open_state("undo-offer");
        let first = state.selected.unwrap();
        state.apply_editor_text("One.".into());
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.apply_editor_text("Two.".into());
        state.add_node(NodeKind::Scene);
        let third = state.selected.unwrap();
        state.select_node(first);
        assert!(!state.undo_offered());

        state.trash_item(second);
        assert!(state.undo_offered());
        state.take_undo_offer();
        assert!(!state.binder.is_in_trash(second));
        assert_eq!(state.binder.scene_after(first), Some(second), "back in its place");
        assert!(state.status.starts_with("Put"), "{}", state.status);
        assert!(!state.undo_offered());

        state.merge_with_next();
        assert!(state.undo_offered());
        state.apply_editor_text("One.
Two.
More.".into());
        assert!(!state.undo_offered(), "typing since the merge hides it");
        state.undo();
        state.status = "Merged".into();
        state.offer_undo(UndoOffer::Structure);
        assert!(state.undo_offered(), "back at the merged text");
        state.take_undo_offer();
        assert!(!state.binder.is_in_trash(second));
        assert_eq!(state.binder.scene_after(second), Some(third));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn picked_scenes_merge_into_the_first_and_one_undo_puts_them_all_back() {
        let (mut state, dir) = open_state("pick");
        let first = state.selected.unwrap();
        state.apply_editor_text("One.".into());
        let mut scenes = vec![first];
        for text in ["Two.", "Three.", "Four."] {
            state.add_node(NodeKind::Scene);
            scenes.push(state.selected.unwrap());
            state.apply_editor_text(text.into());
        }
        state.select_node(scenes[1]);
        state.pick_range(scenes[3]);
        assert_eq!(state.picked, [scenes[1], scenes[2], scenes[3]]);
        state.toggle_pick(scenes[2]);
        assert_eq!(state.picked_with(scenes[1]), [scenes[1], scenes[3]]);
        state.toggle_pick(scenes[2]);
        assert_eq!(state.picked_with(scenes[0]), Vec::<Id>::new(), "a scene not picked has its own menu");

        state.set_picked_status("Revise".into());
        assert!(scenes[1..].iter().all(|id| state.binder.get(*id).unwrap().status == "Revise"));

        state.merge_picked();
        assert_eq!(state.selected, Some(scenes[1]));
        assert_eq!(state.editor_text, "Two.
Three.
Four.");
        assert!(state.binder.is_in_trash(scenes[2]) && state.binder.is_in_trash(scenes[3]));
        assert!(state.picked.is_empty());
        assert!(state.status.starts_with("Merged 2 scenes"), "{}", state.status);

        state.undo();
        assert_eq!(state.editor_text, "Two.");
        assert_eq!(state.reading_order(), scenes, "every scene is back in its place");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_trash_lists_a_trashed_part_as_one_card_with_its_scenes() {
        let (mut state, dir) = open_state("trash-list");
        let scene = state.selected.unwrap();
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        state.trash_item(chapter);
        let trash = state.binder.trash_root().unwrap().id;
        state.choose_library(trash);
        // Under Trash in the binder: the chapter, not each scene in it.
        let rows = state.trash_rows();
        assert_eq!(rows.len(), 1, "the chapter, not each scene in it");
        assert_eq!((rows[0].id, rows[0].depth, rows[0].scenes), (chapter, 1, 1));
        assert!(rows[0].status.is_empty());
        assert!(!state.binder_rows().iter().any(|row| row.id == chapter), "out of the manuscript's rows");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_project_inside_the_open_one_is_refused_and_says_why() {
        let (mut state, dir) = open_state("nested");
        let root = state.project_root().unwrap();
        let title = state.project_title();
        state.create_project_in(root.join("manuscript"));
        assert!(state.status.starts_with("Could not create the project here"), "{}", state.status);
        assert_eq!(state.project_root().unwrap(), root, "the open project stays open");
        assert_eq!(state.project_title(), title);
        assert!(!root.join("manuscript").join("Untitled-Novel.writingstudio").exists());
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
