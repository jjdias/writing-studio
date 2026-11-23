use serde::{Deserialize, Serialize};
use writing_studio_core::{Error, Id, Result, Timestamp};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    DraftRoot,
    ResearchRoot,
    TrashRoot,
    Folder,
    Book,
    Part,
    Chapter,
    Scene,
    Note,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DraftRoot => "draft_root",
            Self::ResearchRoot => "research_root",
            Self::TrashRoot => "trash_root",
            Self::Folder => "folder",
            Self::Book => "book",
            Self::Part => "part",
            Self::Chapter => "chapter",
            Self::Scene => "scene",
            Self::Note => "note",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "draft_root" => Self::DraftRoot,
            "research_root" => Self::ResearchRoot,
            "trash_root" => Self::TrashRoot,
            "folder" => Self::Folder,
            "book" => Self::Book,
            "part" => Self::Part,
            "chapter" => Self::Chapter,
            "scene" => Self::Scene,
            "note" => Self::Note,
            _ => return None,
        })
    }

    pub fn can_hold_document(self) -> bool {
        matches!(self, Self::Scene | Self::Note | Self::Chapter)
    }

    pub fn is_root(self) -> bool {
        matches!(
            self,
            Self::DraftRoot | Self::ResearchRoot | Self::TrashRoot
        )
    }

    pub fn can_contain(self, child: NodeKind) -> bool {
        match self {
            Self::DraftRoot => matches!(child, Self::Book | Self::Folder),
            Self::Book => matches!(child, Self::Part | Self::Chapter | Self::Folder | Self::Scene),
            Self::Part => matches!(child, Self::Part | Self::Chapter | Self::Folder | Self::Scene),
            Self::Chapter => matches!(child, Self::Scene | Self::Folder),
            Self::ResearchRoot => matches!(child, Self::Note | Self::Folder),
            // Location decides whether a folder holds notes or manuscript items.
            Self::Folder => matches!(
                child,
                Self::Folder | Self::Note | Self::Part | Self::Chapter | Self::Scene
            ),
            Self::TrashRoot => !child.is_root(),
            Self::Scene | Self::Note => false,
        }
    }

    pub fn icon_label(self) -> &'static str {
        match self {
            Self::DraftRoot => "Manuscript",
            Self::ResearchRoot => "Research",
            Self::TrashRoot => "Trash",
            Self::Folder => "Folder",
            Self::Book => "Book",
            Self::Part => "Part",
            Self::Chapter => "Chapter",
            Self::Scene => "Scene",
            Self::Note => "Note",
        }
    }
}

/// Where on a binder row an item is dropped: its top edge, its middle, or
/// its bottom edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPlace {
    Before,
    Inside,
    After,
}

impl DropPlace {
    /// The place for a pointer `y` pixels down a row `height` tall. The
    /// middle only means inside for a row that can hold the dragged item;
    /// on any other row the pointer is before or after it.
    pub fn at(y: f64, height: f64, can_hold: bool) -> Self {
        let edge = height / 4.0;
        if can_hold {
            if y < edge {
                Self::Before
            } else if y > height - edge {
                Self::After
            } else {
                Self::Inside
            }
        } else if y < height / 2.0 {
            Self::Before
        } else {
            Self::After
        }
    }
}

enum DropPlan {
    Trash,
    Into(Id),
    Beside { parent: Id, anchor: Id, before: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinderNode {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub kind: NodeKind,
    pub title: String,
    pub sort_index: i64,
    pub document_id: Option<Id>,
    pub synopsis: String,
    pub label: String,
    pub status: String,
    pub color: String,
    /// Keywords the writer gave this item, in the order given. Their colors
    /// are the binder's, one for each keyword.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Left out of compile, with what it holds. Missing on older items,
    /// which are compiled.
    #[serde(default)]
    pub left_out: bool,
    pub deleted_at: Option<Timestamp>,
    pub restore_parent_id: Option<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl BinderNode {
    pub fn new(kind: NodeKind, title: impl Into<String>, parent_id: Option<Id>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        let kind_copy = kind;
        let id = Id::new();
        Self {
            id,
            parent_id,
            kind,
            title: title.into(),
            sort_index,
            // An item's text has the item's own id. Project format 2 stores
            // the two as one file.
            document_id: if kind_copy.can_hold_document() {
                Some(id)
            } else {
                None
            },
            synopsis: String::new(),
            label: String::new(),
            status: String::from("draft"),
            color: String::new(),
            keywords: Vec::new(),
            left_out: false,
            deleted_at: None,
            restore_parent_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::DraftRoot | NodeKind::ResearchRoot | NodeKind::TrashRoot
        )
    }
}

#[derive(Clone, Debug)]
pub struct BinderTree {
    pub nodes: Vec<BinderNode>,
    /// Each keyword in use and its color, one of `BINDER_COLORS`.
    pub keywords: Vec<Keyword>,
    /// Saved filters, each shown as its own row in the library.
    pub filters: Vec<SavedFilter>,
}

impl BinderTree {
    pub fn new(nodes: Vec<BinderNode>) -> Self {
        Self { nodes, keywords: Vec::new(), filters: Vec::new() }
    }

    pub fn children(&self, parent: Option<Id>) -> Vec<&BinderNode> {
        let parent_is_trash = parent
            .and_then(|id| self.get(id))
            .is_some_and(|node| node.kind == NodeKind::TrashRoot);
        let mut kids: Vec<&BinderNode> = self
            .nodes
            .iter()
            .filter(|node| {
                node.parent_id == parent && (parent_is_trash || node.deleted_at.is_none())
            })
            .collect();
        kids.sort_by_key(|node| node.sort_index);
        kids
    }

    pub fn get(&self, id: Id) -> Option<&BinderNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut BinderNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn next_sort_index(&self, parent: Option<Id>) -> i64 {
        self.children(parent)
            .last()
            .map(|node| node.sort_index + 1)
            .unwrap_or(0)
    }

    pub fn draft_root(&self) -> Option<&BinderNode> {
        self.nodes.iter().find(|node| node.kind == NodeKind::DraftRoot)
    }

    /// Books directly under the manuscript, in binder order. Trashed books are omitted.
    pub fn manuscript_books(&self) -> Vec<&BinderNode> {
        let Some(draft) = self.draft_root() else {
            return Vec::new();
        };
        self.children(Some(draft.id))
            .into_iter()
            .filter(|node| node.kind == NodeKind::Book)
            .collect()
    }

    /// The book that contains `id`, when `id` is that book or one of its items.
    pub fn book_of(&self, id: Id) -> Option<Id> {
        let mut current = Some(id);
        let mut guard = 0;
        while let Some(cid) = current {
            let Some(node) = self.get(cid) else {
                return None;
            };
            if node.kind == NodeKind::Book {
                return Some(node.id);
            }
            if node.kind == NodeKind::DraftRoot {
                return None;
            }
            current = node.parent_id;
            guard += 1;
            if guard > 10_000 {
                break;
            }
        }
        None
    }

    pub fn in_manuscript(&self, id: Id) -> bool {
        self.ancestor_is(id, NodeKind::DraftRoot)
    }

    /// Book, chapter, and scene, so the same chapter title in two books stays distinct.
    /// A scene with no chapter uses its parent when that parent is not the book.
    pub fn scene_caption(&self, id: Id) -> String {
        let Some(node) = self.get(id) else {
            return "Missing scene".into();
        };
        let book_id = self.book_of(id);
        let book_title = book_id.and_then(|book| self.get(book)).map(|book| book.title.clone());
        let chapter_title = self
            .nearest_ancestor(id, NodeKind::Chapter)
            .and_then(|chapter| self.get(chapter))
            .map(|chapter| chapter.title.clone());
        let parent = node.parent_id.and_then(|parent| self.get(parent));
        let mut parts = Vec::new();
        if let Some(title) = book_title {
            parts.push(title);
        }
        if let Some(title) = chapter_title {
            parts.push(title);
        } else if let Some(parent) = parent {
            if parent.kind != NodeKind::Book && !parent.kind.is_root() {
                parts.push(parent.title.clone());
            }
        }
        parts.push(node.title.clone());
        let mut label = parts.join(" / ");
        if self.is_in_trash(id) {
            label.push_str(" (Trash)");
        }
        label
    }

    fn nearest_ancestor(&self, id: Id, kind: NodeKind) -> Option<Id> {
        let mut current = self.get(id).and_then(|node| node.parent_id);
        let mut guard = 0;
        while let Some(cid) = current {
            if self.get(cid).is_some_and(|node| node.kind == kind) {
                return Some(cid);
            }
            current = self.get(cid).and_then(|node| node.parent_id);
            guard += 1;
            if guard > 10_000 {
                break;
            }
        }
        None
    }

    pub fn research_root(&self) -> Option<&BinderNode> {
        self.nodes
            .iter()
            .find(|node| node.kind == NodeKind::ResearchRoot)
    }

    pub fn trash_root(&self) -> Option<&BinderNode> {
        self.nodes.iter().find(|node| node.kind == NodeKind::TrashRoot)
    }

    pub fn is_in_trash(&self, id: Id) -> bool {
        self.ancestor_is(id, NodeKind::TrashRoot)
    }

    pub fn is_in_research(&self, id: Id) -> bool {
        self.ancestor_is(id, NodeKind::ResearchRoot)
    }

    fn ancestor_is(&self, id: Id, kind: NodeKind) -> bool {
        let mut current = Some(id);
        let mut guard = 0;
        while let Some(cid) = current {
            if self.get(cid).is_some_and(|node| node.kind == kind) {
                return true;
            }
            current = self.get(cid).and_then(|node| node.parent_id);
            guard += 1;
            if guard > 10_000 {
                break;
            }
        }
        false
    }

    /// Whether `parent` may hold a new child of `child`, including folder context.
    pub fn accepts(&self, parent: Id, child: NodeKind) -> bool {
        let Some(node) = self.get(parent) else {
            return false;
        };
        if node.deleted_at.is_some() || (self.is_in_trash(parent) && node.kind != NodeKind::TrashRoot) {
            return false;
        }
        if node.kind == NodeKind::Folder && self.is_in_research(parent) {
            return matches!(child, NodeKind::Note | NodeKind::Folder);
        }
        if node.kind == NodeKind::Folder {
            return matches!(
                child,
                NodeKind::Folder | NodeKind::Part | NodeKind::Chapter | NodeKind::Scene
            );
        }
        node.kind.can_contain(child)
    }

    /// Nearest container at or above `start` that can hold `child`.
    pub fn container_for(&self, start: Id, child: NodeKind) -> Option<Id> {
        let mut current = Some(start);
        let mut guard = 0;
        while let Some(id) = current {
            if self.accepts(id, child) {
                return Some(id);
            }
            current = self.get(id).and_then(|node| node.parent_id);
            guard += 1;
            if guard > 10_000 {
                break;
            }
        }
        None
    }

    pub fn drop_allowed(&self, dragged: Id, target: Id) -> bool {
        if dragged == target || self.is_under(target, dragged) {
            return false;
        }
        let Some(drag_kind) = self.get(dragged).map(|node| node.kind) else {
            return false;
        };
        if drag_kind.is_root() {
            return false;
        }
        let Some(target_node) = self.get(target) else {
            return false;
        };
        if target_node.kind == NodeKind::TrashRoot {
            return true;
        }
        if self.accepts(target, drag_kind) {
            return true;
        }
        target_node
            .parent_id
            .is_some_and(|parent| self.accepts(parent, drag_kind))
    }

    fn is_under(&self, node: Id, ancestor: Id) -> bool {
        let mut current = Some(node);
        let mut guard = 0;
        while let Some(cid) = current {
            if cid == ancestor {
                return true;
            }
            current = self.get(cid).and_then(|n| n.parent_id);
            guard += 1;
            if guard > 10_000 {
                break;
            }
        }
        false
    }

    /// Drop `dragged` onto `target`: nest, reorder, or move to trash.
    pub fn drop_on(&mut self, dragged: Id, target: Id) -> Result<()> {
        if dragged == target {
            return Err(Error::message("cannot drop an item on itself"));
        }
        if self.is_under(target, dragged) {
            return Err(Error::message("cannot drop an item into its own contents"));
        }
        let drag_kind = self
            .get(dragged)
            .map(|node| node.kind)
            .ok_or_else(|| Error::NodeNotFound(dragged.to_string()))?;
        if drag_kind.is_root() {
            return Err(Error::message("manuscript, research, and trash cannot be moved"));
        }
        let target_node = self
            .get(target)
            .cloned()
            .ok_or_else(|| Error::NodeNotFound(target.to_string()))?;

        if target_node.kind == NodeKind::TrashRoot {
            return self.trash(dragged);
        }
        if self.accepts(target, drag_kind) {
            return self.place(dragged, Some(target), None);
        }
        if target_node
            .parent_id
            .is_some_and(|parent| self.accepts(parent, drag_kind))
        {
            return self.place(dragged, target_node.parent_id, Some(target));
        }
        Err(Error::message("that binder item cannot hold the dragged item"))
    }

    /// The manuscript's books, parts, chapters, and folders in binder order,
    /// each with its depth below the manuscript. The library column lists
    /// these; scenes are listed beside it.
    pub fn manuscript_containers(&self) -> Vec<(Id, u32)> {
        let mut rows = Vec::new();
        if let Some(draft) = self.draft_root() {
            self.collect_containers(draft.id, 0, &mut rows);
        }
        rows
    }

    fn collect_containers(&self, parent: Id, depth: u32, rows: &mut Vec<(Id, u32)>) {
        for child in self.children(Some(parent)) {
            if matches!(child.kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Folder) {
                rows.push((child.id, depth));
                self.collect_containers(child.id, depth + 1, rows);
            }
        }
    }

    /// Every scene below `container`, in binder order, each with the item
    /// that directly holds it, so a list can group scenes by chapter.
    pub fn scenes_under(&self, container: Id) -> Vec<(Id, Id)> {
        let mut scenes = Vec::new();
        self.collect_scenes(container, &mut scenes);
        scenes
    }

    /// Whether compile leaves `id` out: it was left out, or something it is
    /// in was.
    pub fn is_left_out(&self, id: Id) -> bool {
        let mut at = Some(id);
        let mut guard = 0;
        while let Some(current) = at.and_then(|id| self.get(id)) {
            if current.left_out {
                return true;
            }
            at = current.parent_id;
            guard += 1;
            if guard > 256 {
                return false;
            }
        }
        false
    }

    /// The scene after `id` in reading order, in the same book, even when it
    /// is in the next chapter. Merging joins it into `id`.
    pub fn scene_after(&self, id: Id) -> Option<Id> {
        let book = self.book_of(id)?;
        let scenes: Vec<Id> = self
            .scenes_under(book)
            .into_iter()
            .map(|(_, scene)| scene)
            .filter(|scene| self.get(*scene).is_some_and(|node| node.kind == NodeKind::Scene))
            .collect();
        let at = scenes.iter().position(|scene| *scene == id)?;
        scenes.get(at + 1).copied()
    }

    /// The item just before `id` under the same parent.
    pub fn previous_sibling(&self, id: Id) -> Option<Id> {
        let parent = self.get(id)?.parent_id;
        let siblings: Vec<Id> = self.children(parent).into_iter().map(|node| node.id).collect();
        let at = siblings.iter().position(|sibling| *sibling == id)?;
        at.checked_sub(1).map(|before| siblings[before])
    }

    /// Put `id` under `parent` right after `after`, or first when `after` is
    /// `None`. Out of the trash too, for undoing a merge.
    pub fn put_after(&mut self, id: Id, parent: Id, after: Option<Id>) -> Result<()> {
        match after {
            Some(after) => self.place(id, Some(parent), Some(after)),
            None => self.place_first(id, parent),
        }
    }

    fn collect_scenes(&self, parent: Id, scenes: &mut Vec<(Id, Id)>) {
        for child in self.children(Some(parent)) {
            if matches!(child.kind, NodeKind::Scene | NodeKind::Note) {
                scenes.push((parent, child.id));
            }
            self.collect_scenes(child.id, scenes);
        }
    }

    /// Where a drop at `place` on `target` puts `dragged`, or why it cannot.
    /// Before and after go next to the target, or next to the nearest
    /// container above it whose parent can hold the dragged kind, so a
    /// chapter dropped on a scene lands beside that scene's chapter. Inside
    /// a row that cannot hold the dragged item means after it.
    fn resolve_drop(&self, dragged: Id, target: Id, place: DropPlace) -> Result<DropPlan> {
        if dragged == target {
            return Err(Error::message("cannot drop an item on itself"));
        }
        if self.is_under(target, dragged) {
            return Err(Error::message("cannot drop an item into its own contents"));
        }
        let drag_kind = self
            .get(dragged)
            .map(|node| node.kind)
            .ok_or_else(|| Error::NodeNotFound(dragged.to_string()))?;
        if drag_kind.is_root() {
            return Err(Error::message("manuscript, research, and trash cannot be moved"));
        }
        let target_node = self
            .get(target)
            .ok_or_else(|| Error::NodeNotFound(target.to_string()))?;
        if target_node.kind == NodeKind::TrashRoot {
            return Ok(DropPlan::Trash);
        }
        if place == DropPlace::Inside && self.accepts(target, drag_kind) {
            return Ok(DropPlan::Into(target));
        }
        if target_node.kind.is_root() {
            return if self.accepts(target, drag_kind) {
                Ok(DropPlan::Into(target))
            } else {
                Err(Error::message("that binder item cannot hold the dragged item"))
            };
        }
        let mut anchor = target;
        let mut guard = 0;
        loop {
            let Some(parent) = self.get(anchor).and_then(|node| node.parent_id) else {
                return Err(Error::message("that binder item cannot hold the dragged item"));
            };
            if self.accepts(parent, drag_kind) {
                return Ok(DropPlan::Beside {
                    parent,
                    anchor,
                    before: place == DropPlace::Before,
                });
            }
            if self.get(parent).is_some_and(|node| node.kind.is_root()) {
                return Err(Error::message("that binder item cannot hold the dragged item"));
            }
            anchor = parent;
            guard += 1;
            if guard > 10_000 {
                return Err(Error::message("the binder is nested too deeply"));
            }
        }
    }

    pub fn drop_allowed_at(&self, dragged: Id, target: Id, place: DropPlace) -> bool {
        self.resolve_drop(dragged, target, place).is_ok()
    }

    /// Drop `dragged` before, inside, or after `target`, anywhere in the
    /// binder its kind can go.
    pub fn drop_at(&mut self, dragged: Id, target: Id, place: DropPlace) -> Result<()> {
        match self.resolve_drop(dragged, target, place)? {
            DropPlan::Trash => self.trash(dragged),
            DropPlan::Into(parent) => self.place(dragged, Some(parent), None),
            DropPlan::Beside { parent, anchor, before: false } => {
                self.place(dragged, Some(parent), Some(anchor))
            }
            DropPlan::Beside { parent, anchor, before: true } => {
                let siblings: Vec<Id> = self
                    .children(Some(parent))
                    .into_iter()
                    .map(|node| node.id)
                    .filter(|id| *id != dragged)
                    .collect();
                let index = siblings.iter().position(|id| *id == anchor).unwrap_or(0);
                match index.checked_sub(1).map(|previous| siblings[previous]) {
                    Some(previous) => self.place(dragged, Some(parent), Some(previous)),
                    None => self.place_first(dragged, parent),
                }
            }
        }
    }

    /// Put `dragged` first under `parent`.
    fn place_first(&mut self, dragged: Id, parent: Id) -> Result<()> {
        self.place(dragged, Some(parent), None)?;
        let mut order: Vec<Id> = self
            .children(Some(parent))
            .into_iter()
            .map(|node| node.id)
            .filter(|id| *id != dragged)
            .collect();
        order.insert(0, dragged);
        for (index, id) in order.iter().enumerate() {
            if let Some(node) = self.get_mut(*id) {
                node.sort_index = index as i64;
            }
        }
        Ok(())
    }

    pub fn trash(&mut self, id: Id) -> Result<()> {
        let node = self
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NodeNotFound(id.to_string()))?;
        if node.kind.is_root() {
            return Err(Error::message("cannot trash a binder root"));
        }
        let trash_id = self
            .trash_root()
            .map(|root| root.id)
            .ok_or_else(|| Error::message("project has no trash"))?;
        if self.is_in_trash(id) {
            return Ok(());
        }
        let old_parent = node.parent_id;
        if let Some(entry) = self.get_mut(id) {
            entry.restore_parent_id = old_parent;
            entry.parent_id = Some(trash_id);
            entry.deleted_at = Some(Timestamp::now());
            entry.updated_at = Timestamp::now();
        }
        self.reindex(old_parent);
        self.reindex(Some(trash_id));
        Ok(())
    }

    pub fn restore(&mut self, id: Id) -> Result<()> {
        let node = self
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NodeNotFound(id.to_string()))?;
        if !self.is_in_trash(id) && node.deleted_at.is_none() {
            return Err(Error::message("item is not in the trash"));
        }
        let dest = node
            .restore_parent_id
            .filter(|parent| {
                self.get(*parent)
                    .is_some_and(|candidate| {
                        candidate.deleted_at.is_none()
                            && !self.is_in_trash(*parent)
                            && self.accepts(*parent, node.kind)
                    })
            })
            .or_else(|| self.default_parent(node.kind));
        self.place(id, dest, None)
    }

    pub fn default_parent(&self, kind: NodeKind) -> Option<Id> {
        match kind {
            NodeKind::Note => self.research_root().map(|node| node.id),
            NodeKind::Folder => self
                .manuscript_books()
                .first()
                .map(|book| book.id)
                .or_else(|| self.draft_root().map(|node| node.id)),
            NodeKind::Scene => self
                .nodes
                .iter()
                .rev()
                .find(|node| {
                    node.deleted_at.is_none()
                        && !self.is_in_trash(node.id)
                        && !self.is_in_research(node.id)
                        && self.accepts(node.id, NodeKind::Scene)
                })
                .map(|node| node.id),
            NodeKind::Chapter | NodeKind::Part => self
                .nodes
                .iter()
                .find(|node| node.kind == NodeKind::Book && node.deleted_at.is_none())
                .map(|node| node.id),
            NodeKind::Book => self.draft_root().map(|node| node.id),
            _ => self.draft_root().map(|node| node.id),
        }
    }

    /// Notes under the research root, outside the trash, parents before nested notes.
    pub fn research_notes(&self) -> Vec<Id> {
        let Some(root) = self.research_root() else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        self.collect_research(root.id, NodeKind::Note, &mut notes);
        notes
    }

    /// Folders under the research root, outside the trash, children before parents.
    pub fn research_folders(&self) -> Vec<Id> {
        let Some(root) = self.research_root() else {
            return Vec::new();
        };
        let mut folders = Vec::new();
        self.collect_research(root.id, NodeKind::Folder, &mut folders);
        folders
    }

    fn collect_research(&self, parent: Id, kind: NodeKind, found: &mut Vec<Id>) {
        for child in self.children(Some(parent)) {
            if child.kind == NodeKind::Folder {
                self.collect_research(child.id, kind, found);
            }
            if child.kind == kind {
                found.push(child.id);
            }
        }
    }

    /// Where a new binder item belongs.
    ///
    /// With the whole series on screen (`book_scope` is none), follow `selected`, then [`Self::default_parent`].
    /// With one book on screen, chapters, parts, scenes, and manuscript folders stay inside that book.
    /// A selection in another book is ignored. Notes and new books keep their usual parents.
    pub fn parent_for_new_item(
        &self,
        kind: NodeKind,
        selected: Option<Id>,
        book_scope: Option<Id>,
    ) -> Option<Id> {
        let scoped = book_scope.filter(|book| {
            self.manuscript_books().iter().any(|candidate| candidate.id == *book)
        });
        let inside_book = scoped.is_some()
            && matches!(
                kind,
                NodeKind::Chapter | NodeKind::Part | NodeKind::Scene | NodeKind::Folder
            );
        if !inside_book {
            if let Some(id) = selected.and_then(|id| self.container_for(id, kind)) {
                return Some(id);
            }
            return self.default_parent(kind);
        }
        let book = scoped?;
        if let Some(selected) = selected {
            if self.book_of(selected) == Some(book) {
                if let Some(parent) = self.container_for(selected, kind) {
                    if parent == book || self.book_of(parent) == Some(book) {
                        return Some(parent);
                    }
                }
            }
        }
        match kind {
            NodeKind::Scene => self.last_chapter_in(book).or(Some(book)),
            NodeKind::Chapter | NodeKind::Part | NodeKind::Folder => Some(book),
            _ => Some(book),
        }
    }

    /// The last chapter under `book`, in binder order. Chapters nested in parts and folders count.
    pub fn last_chapter_in(&self, book: Id) -> Option<Id> {
        let mut last = None;
        self.walk_chapters(book, &mut last);
        last
    }

    fn walk_chapters(&self, parent: Id, last: &mut Option<Id>) {
        for child in self.children(Some(parent)) {
            if child.kind == NodeKind::Chapter {
                *last = Some(child.id);
            }
            if matches!(
                child.kind,
                NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Folder
            ) {
                self.walk_chapters(child.id, last);
            }
        }
    }

    /// Permanently remove trashed items (not the trash root).
    /// Returns `(removed node ids, document ids to delete)`.
    pub fn purge_trash(&mut self) -> (Vec<Id>, Vec<Id>) {
        let ids: Vec<Id> = self
            .nodes
            .iter()
            .filter(|node| !node.kind.is_root() && self.is_in_trash(node.id))
            .map(|node| node.id)
            .collect();
        let documents: Vec<Id> = ids
            .iter()
            .filter_map(|id| self.get(*id).and_then(|node| node.document_id))
            .collect();
        self.nodes.retain(|node| !ids.contains(&node.id));
        if let Some(trash_id) = self.trash_root().map(|node| node.id) {
            self.reindex(Some(trash_id));
        }
        (ids, documents)
    }

    fn place(&mut self, dragged: Id, new_parent: Option<Id>, after: Option<Id>) -> Result<()> {
        let old_parent = self.get(dragged).and_then(|node| node.parent_id);
        if let Some(node) = self.get_mut(dragged) {
            node.parent_id = new_parent;
            node.deleted_at = None;
            node.restore_parent_id = None;
            node.updated_at = Timestamp::now();
        } else {
            return Err(Error::NodeNotFound(dragged.to_string()));
        }

        let mut order: Vec<Id> = self
            .children(new_parent)
            .into_iter()
            .map(|node| node.id)
            .filter(|id| *id != dragged)
            .collect();
        if let Some(after_id) = after {
            if let Some(index) = order.iter().position(|id| *id == after_id) {
                order.insert(index + 1, dragged);
            } else {
                order.push(dragged);
            }
        } else {
            order.push(dragged);
        }
        for (index, id) in order.iter().enumerate() {
            if let Some(node) = self.get_mut(*id) {
                node.sort_index = index as i64;
                node.updated_at = Timestamp::now();
            }
        }
        if old_parent != new_parent {
            self.reindex(old_parent);
        }
        Ok(())
    }

    fn reindex(&mut self, parent: Option<Id>) {
        let ids: Vec<Id> = self.children(parent).into_iter().map(|node| node.id).collect();
        for (index, id) in ids.iter().enumerate() {
            if let Some(node) = self.get_mut(*id) {
                node.sort_index = index as i64;
                node.updated_at = Timestamp::now();
            }
        }
    }

    pub fn total_word_count<F>(&self, mut words_for: F) -> u32
    where
        F: FnMut(Id) -> u32,
    {
        self.nodes
            .iter()
            .filter(|node| node.deleted_at.is_none() && !self.is_in_trash(node.id))
            .filter_map(|node| node.document_id)
            .map(|id| words_for(id))
            .sum()
    }

    /// Word count of one binder item and the items nested under it.
    pub fn subtree_word_count<F>(&self, root: Id, mut words_for: F) -> u32
    where
        F: FnMut(Id) -> u32,
    {
        fn walk<F>(tree: &BinderTree, id: Id, words_for: &mut F) -> u32
        where
            F: FnMut(Id) -> u32,
        {
            let Some(node) = tree.get(id) else {
                return 0;
            };
            if node.deleted_at.is_some() || (tree.is_in_trash(id) && !node.kind.is_root()) {
                return 0;
            }
            let mut total = node.document_id.map(&mut *words_for).unwrap_or(0);
            for child in tree.children(Some(id)) {
                total = total.saturating_add(walk(tree, child.id, words_for));
            }
            total
        }
        walk(self, root, &mut words_for)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> (BinderTree, Id, Id, Id, Id) {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let research = BinderNode::new(NodeKind::ResearchRoot, "Research", None, 1);
        let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 2);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let scene_a = BinderNode::new(NodeKind::Scene, "Scene A", Some(chapter.id), 0);
        let scene_b = BinderNode::new(NodeKind::Scene, "Scene B", Some(chapter.id), 1);
        let chapter_id = chapter.id;
        let a = scene_a.id;
        let b = scene_b.id;
        let trash_id = trash.id;
        let binder = BinderTree::new(vec![
            draft, research, trash, book, chapter, scene_a, scene_b,
        ]);
        (binder, chapter_id, a, b, trash_id)
    }

    fn titles(binder: &BinderTree, parent: Id) -> Vec<String> {
        binder
            .children(Some(parent))
            .into_iter()
            .map(|node| node.title.clone())
            .collect()
    }

    #[test]
    fn a_scene_goes_before_after_or_into_wherever_it_is_dropped() {
        let (mut binder, chapter, a, b, _) = tree();
        let book = binder.get(chapter).unwrap().parent_id.unwrap();
        // Before the first scene: the spot drop_on could not reach.
        binder.drop_at(b, a, DropPlace::Before).unwrap();
        assert_eq!(titles(&binder, chapter), ["Scene B", "Scene A"]);
        // Into another chapter, then before its only scene.
        let second = BinderNode::new(NodeKind::Chapter, "Chapter 2", Some(book), 1);
        let second_id = second.id;
        binder.nodes.push(second);
        binder.drop_at(a, second_id, DropPlace::Inside).unwrap();
        assert_eq!(titles(&binder, second_id), ["Scene A"]);
        binder.drop_at(b, a, DropPlace::Before).unwrap();
        assert_eq!(titles(&binder, second_id), ["Scene B", "Scene A"]);
        assert!(titles(&binder, chapter).is_empty());
        // After a chapter's row edge is beside the chapter, not in it.
        binder.drop_at(a, chapter, DropPlace::After).unwrap();
        assert_eq!(titles(&binder, book), ["Chapter 1", "Scene A", "Chapter 2"]);
    }

    #[test]
    fn a_chapter_dropped_on_a_scene_lands_beside_that_scenes_chapter() {
        let (mut binder, chapter, a, _, _) = tree();
        let book = binder.get(chapter).unwrap().parent_id.unwrap();
        let second = BinderNode::new(NodeKind::Chapter, "Chapter 2", Some(book), 1);
        let second_id = second.id;
        binder.nodes.push(second);
        assert!(binder.drop_allowed_at(second_id, a, DropPlace::Before));
        binder.drop_at(second_id, a, DropPlace::Before).unwrap();
        assert_eq!(titles(&binder, book), ["Chapter 2", "Chapter 1"]);
        // A chapter moves to another book the same way.
        let draft = binder.draft_root().unwrap().id;
        let other = BinderNode::new(NodeKind::Book, "Book 2", Some(draft), 1);
        let other_id = other.id;
        binder.nodes.push(other);
        binder.drop_at(chapter, other_id, DropPlace::Inside).unwrap();
        assert_eq!(titles(&binder, other_id), ["Chapter 1"]);
        // Never into its own contents, and roots stay put.
        assert!(!binder.drop_allowed_at(chapter, a, DropPlace::Inside));
        let trash = binder.trash_root().unwrap().id;
        assert!(!binder.drop_allowed_at(trash, a, DropPlace::After));
    }

    #[test]
    fn the_library_lists_containers_and_the_list_their_scenes_by_chapter() {
        let (mut binder, chapter, a, b, _) = tree();
        let book = binder.get(chapter).unwrap().parent_id.unwrap();
        let part = BinderNode::new(NodeKind::Part, "Part 2", Some(book), 1);
        let part_id = part.id;
        let inner = BinderNode::new(NodeKind::Chapter, "Chapter 2", Some(part_id), 0);
        let inner_id = inner.id;
        let c = BinderNode::new(NodeKind::Scene, "Scene C", Some(inner_id), 0);
        let c_id = c.id;
        binder.nodes.extend([part, inner, c]);
        assert_eq!(
            binder.manuscript_containers(),
            [(book, 0), (chapter, 1), (part_id, 1), (inner_id, 2)]
        );
        assert_eq!(
            binder.scenes_under(book),
            [(chapter, a), (chapter, b), (inner_id, c_id)]
        );
        assert_eq!(binder.scenes_under(inner_id), [(inner_id, c_id)]);
        // Scenes in the trash are listed under the trash, not the book.
        binder.trash(a).unwrap();
        let trash = binder.trash_root().unwrap().id;
        assert_eq!(binder.scenes_under(trash), [(trash, a)]);
        assert_eq!(binder.scenes_under(book).len(), 2);
    }

    #[test]
    fn the_middle_of_a_row_is_inside_only_when_it_can_hold_the_item() {
        assert_eq!(DropPlace::at(2.0, 26.0, true), DropPlace::Before);
        assert_eq!(DropPlace::at(13.0, 26.0, true), DropPlace::Inside);
        assert_eq!(DropPlace::at(24.0, 26.0, true), DropPlace::After);
        assert_eq!(DropPlace::at(10.0, 26.0, false), DropPlace::Before);
        assert_eq!(DropPlace::at(16.0, 26.0, false), DropPlace::After);
    }

    #[test]
    fn reorder_scene_after_sibling() {
        let (mut binder, _, a, b, _) = tree();
        binder.drop_on(a, b).unwrap();
        let order: Vec<_> = binder
            .children(binder.get(a).and_then(|node| node.parent_id))
            .into_iter()
            .map(|node| node.title.as_str())
            .collect();
        assert_eq!(order, vec!["Scene B", "Scene A"]);
        binder.drop_on(b, a).unwrap();
        let order: Vec<_> = binder
            .children(binder.get(a).and_then(|node| node.parent_id))
            .into_iter()
            .map(|node| node.title.as_str())
            .collect();
        assert_eq!(order, vec!["Scene A", "Scene B"]);
    }

    #[test]
    fn trash_and_restore_keeps_previous_parent() {
        let (mut binder, chapter, a, _, trash_id) = tree();
        binder.trash(a).unwrap();
        assert!(binder.is_in_trash(a));
        assert_eq!(binder.get(a).unwrap().parent_id, Some(trash_id));
        assert!(binder.children(Some(chapter)).iter().all(|node| node.id != a));
        binder.restore(a).unwrap();
        assert!(!binder.is_in_trash(a));
        assert_eq!(binder.get(a).unwrap().parent_id, Some(chapter));
        assert!(binder.get(a).unwrap().deleted_at.is_none());
    }

    #[test]
    fn cannot_drop_into_own_descendant() {
        let (mut binder, chapter, a, _, _) = tree();
        let err = binder.drop_on(chapter, a).unwrap_err();
        assert!(err.to_string().contains("own contents"));
    }

    #[test]
    fn purge_removes_trashed_documents() {
        let (mut binder, _, a, _, _) = tree();
        let doc = binder.get(a).unwrap().document_id.unwrap();
        binder.trash(a).unwrap();
        let (nodes, docs) = binder.purge_trash();
        assert!(nodes.contains(&a));
        assert!(docs.contains(&doc));
        assert!(binder.get(a).is_none());
    }

    #[test]
    fn manuscript_folder_nests_chapters_and_scenes() {
        let (mut binder, chapter, scene, _, _) = tree();
        let book = binder.get(chapter).unwrap().parent_id.unwrap();
        let mut folder = BinderNode::new(NodeKind::Folder, "Act folder", Some(book), 1);
        let folder_id = folder.id;
        folder.parent_id = Some(book);
        binder.nodes.push(folder);
        binder.drop_on(chapter, folder_id).unwrap();
        assert_eq!(binder.get(chapter).unwrap().parent_id, Some(folder_id));
        let nested = BinderNode::new(NodeKind::Folder, "Scene group", Some(chapter), 2);
        let nested_id = nested.id;
        binder.nodes.push(nested);
        binder.drop_on(scene, nested_id).unwrap();
        assert_eq!(binder.get(scene).unwrap().parent_id, Some(nested_id));
        let research = binder.research_root().unwrap().id;
        let notes = BinderNode::new(NodeKind::Folder, "Clippings", Some(research), 0);
        let notes_id = notes.id;
        binder.nodes.push(notes);
        let err = binder.drop_on(chapter, notes_id).unwrap_err();
        assert!(err.to_string().contains("cannot hold"));
    }

    #[test]
    fn parts_can_nest() {
        let (mut binder, chapter, _, _, _) = tree();
        let book = binder.get(chapter).unwrap().parent_id.unwrap();
        let outer = BinderNode::new(NodeKind::Part, "Part 1", Some(book), 1);
        let outer_id = outer.id;
        binder.nodes.push(outer);
        let inner = BinderNode::new(NodeKind::Part, "Part 1a", Some(book), 2);
        let inner_id = inner.id;
        binder.nodes.push(inner);
        assert!(binder.drop_allowed(inner_id, outer_id));
        binder.drop_on(inner_id, outer_id).unwrap();
        assert_eq!(binder.get(inner_id).unwrap().parent_id, Some(outer_id));
        assert_eq!(binder.container_for(inner_id, NodeKind::Chapter), Some(inner_id));
    }

    #[test]
    fn each_book_keeps_its_own_word_count() {
        let (mut binder, chapter, scene, _, _) = tree();
        let draft = binder.draft_root().unwrap().id;
        let first = binder.get(chapter).unwrap().parent_id.unwrap();
        let second = BinderNode::new(NodeKind::Book, "Second", Some(draft), 1);
        let second_id = second.id;
        let scene_two = BinderNode::new(NodeKind::Scene, "Other scene", Some(second_id), 0);
        let other_doc = scene_two.document_id.unwrap();
        binder.nodes.push(second);
        binder.nodes.push(scene_two);
        let first_doc = binder.get(scene).unwrap().document_id.unwrap();
        let words = |id: Id| {
            if id == first_doc {
                12
            } else if id == other_doc {
                7
            } else {
                1
            }
        };
        assert_eq!(binder.manuscript_books().len(), 2);
        assert_eq!(binder.book_of(scene), Some(first));
        assert_eq!(binder.book_of(chapter), Some(first));
        assert_eq!(binder.subtree_word_count(first, words), 14);
        assert_eq!(binder.subtree_word_count(second_id, words), 7);
        let research = binder.research_root().unwrap().id;
        assert!(!binder.in_manuscript(research));
        assert!(binder.in_manuscript(first));
    }

    #[test]
    fn a_filtered_book_receives_new_items_instead_of_the_first_book() {
        let (binder, chapter, scene, _, _) = tree();
        let first = binder.get(chapter).unwrap().parent_id.unwrap();
        let draft = binder.draft_root().unwrap().id;
        let mut binder = binder;
        let second = BinderNode::new(NodeKind::Book, "Second", Some(draft), 1);
        let second_id = second.id;
        let early = BinderNode::new(NodeKind::Chapter, "Early", Some(second_id), 0);
        let early_id = early.id;
        let later = BinderNode::new(NodeKind::Chapter, "Later", Some(second_id), 1);
        let later_id = later.id;
        binder.nodes.push(second);
        binder.nodes.push(early);
        binder.nodes.push(later);

        assert_eq!(
            binder.parent_for_new_item(NodeKind::Chapter, Some(scene), None),
            Some(first)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Chapter, Some(scene), Some(second_id)),
            Some(second_id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Scene, None, Some(second_id)),
            Some(later_id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Scene, Some(early_id), Some(second_id)),
            Some(early_id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Folder, Some(scene), Some(second_id)),
            Some(second_id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Part, Some(later_id), Some(second_id)),
            Some(second_id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Note, Some(later_id), Some(second_id)),
            binder.research_root().map(|node| node.id)
        );
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Folder, None, None),
            Some(first)
        );
        let research = binder.research_root().unwrap().id;
        let clippings = BinderNode::new(NodeKind::Folder, "Clippings", Some(research), 0);
        let clippings_id = clippings.id;
        let map = BinderNode::new(NodeKind::Note, "Map", Some(clippings_id), 0);
        let map_id = map.id;
        let loose = BinderNode::new(NodeKind::Note, "Loose", Some(research), 1);
        let loose_id = loose.id;
        binder.nodes.push(clippings);
        binder.nodes.push(map);
        binder.nodes.push(loose);
        binder.trash(loose_id).unwrap();
        assert_eq!(binder.research_notes(), vec![map_id]);
        assert_eq!(binder.research_folders(), vec![clippings_id]);
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Book, None, Some(second_id)),
            Some(draft)
        );
        let part = BinderNode::new(NodeKind::Part, "Part", Some(second_id), 2);
        let part_id = part.id;
        let nested = BinderNode::new(NodeKind::Chapter, "Nested", Some(part_id), 0);
        let nested_id = nested.id;
        binder.nodes.push(part);
        binder.nodes.push(nested);
        assert_eq!(binder.last_chapter_in(second_id), Some(nested_id));
        assert_eq!(binder.scene_caption(scene), format!("Book / Chapter 1 / {}", binder.get(scene).unwrap().title));
        let other = BinderNode::new(NodeKind::Scene, "Tide", Some(nested_id), 0);
        let other_id = other.id;
        binder.nodes.push(other);
        assert_eq!(binder.scene_caption(other_id), "Second / Nested / Tide");
        let loose = BinderNode::new(NodeKind::Scene, "Loose", Some(second_id), 3);
        let loose_id = loose.id;
        binder.nodes.push(loose);
        assert_eq!(binder.scene_caption(loose_id), "Second / Loose");
        assert_eq!(binder.scene_caption(Id::new()), "Missing scene");
        assert_eq!(
            binder.parent_for_new_item(NodeKind::Scene, None, Some(second_id)),
            Some(nested_id)
        );
    }
}

/// Named binder colors. `id` is what projects store; `hex` is only for display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinderColor {
    pub id: &'static str,
    pub name: &'static str,
    pub hex: &'static str,
}

pub const BINDER_COLORS: &[BinderColor] = &[
    BinderColor { id: "", name: "None", hex: "" },
    BinderColor { id: "gold", name: "Gold", hex: "#c4a265" },
    BinderColor { id: "green", name: "Green", hex: "#6f8f72" },
    BinderColor { id: "red", name: "Red", hex: "#b85c38" },
    BinderColor { id: "blue", name: "Blue", hex: "#3d6b8a" },
    BinderColor { id: "violet", name: "Violet", hex: "#7a6a9a" },
];

pub fn normalize_binder_color(value: &str) -> String {
    let value = value.trim();
    if BINDER_COLORS.iter().any(|swatch| swatch.id == value) {
        value.to_string()
    } else {
        String::new()
    }
}

pub fn binder_color_hex(value: &str) -> &'static str {
    BINDER_COLORS
        .iter()
        .find(|swatch| swatch.id == value.trim())
        .map(|swatch| swatch.hex)
        .unwrap_or("")
}

/// A keyword and its color, shared by every item that carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyword {
    pub name: String,
    /// One of `BINDER_COLORS`' ids.
    pub color: String,
}

/// A filter kept as a row of the library: the scenes that carry its
/// keywords (all of them, or any with `any`) and none of `without`, have
/// its status, and contain its text. Keywords count through the chapter,
/// part, or book a scene is in, and take in what is nested under them. An
/// empty field matches anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedFilter {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub any: bool,
    #[serde(default)]
    pub without: Vec<String>,
    /// Story entries (characters, places, notes) that must reach the scene,
    /// by id. Matched by the caller, which has the bible.
    #[serde(default)]
    pub entries: Vec<Id>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub text: String,
}

/// A binder search split into its keywords (`#plot`), the keywords it
/// leaves out (`-#plot`), and the words looked for in titles and text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeywordQuery {
    pub with: Vec<String>,
    pub without: Vec<String>,
    pub text: String,
}

impl KeywordQuery {
    /// `#plot -#draft harbor` asks for `plot`, leaves out `draft`, and looks
    /// for "harbor". A `#` with no keyword after it is text.
    pub fn parse(query: &str) -> Self {
        let mut parsed = Self::default();
        let mut words = Vec::new();
        for word in query.split_whitespace() {
            let (list, rest) = match word.strip_prefix("-#") {
                Some(rest) => (&mut parsed.without, rest),
                None => match word.strip_prefix('#') {
                    Some(rest) => (&mut parsed.with, rest),
                    None => {
                        words.push(word);
                        continue;
                    }
                },
            };
            match tidy_keyword(rest) {
                Some(keyword) => list.push(keyword),
                None => words.push(word),
            }
        }
        parsed.text = words.join(" ");
        parsed
    }
}

/// How the library's keyword list orders the keywords at each level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeywordSort {
    #[default]
    Name,
    /// Most used first, then by name.
    Count,
}

/// One row of the library's keyword list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeywordRow {
    /// The whole keyword, `plot/subplot`.
    pub name: String,
    /// What the row shows, the last part: `subplot`.
    pub leaf: String,
    pub depth: usize,
    /// Items outside the trash carrying it or one nested under it.
    pub count: usize,
    pub has_children: bool,
}

struct KeywordEntry {
    name: String,
    parent: Option<String>,
    count: usize,
}

/// The keywords nested directly in `parent` (the top level for `None`),
/// sorted, each followed by its own children.
fn push_keyword_level(
    entries: &[KeywordEntry],
    parent: Option<&str>,
    depth: usize,
    sort: KeywordSort,
    rows: &mut Vec<KeywordRow>,
) {
    let in_parent = |entry: &KeywordEntry, parent: Option<&str>| match (entry.parent.as_deref(), parent) {
        (None, None) => true,
        (Some(up), Some(parent)) => same_word(up, parent),
        _ => false,
    };
    let mut here: Vec<&KeywordEntry> = entries.iter().filter(|entry| in_parent(entry, parent)).collect();
    here.sort_by(|one, other| {
        let by_name = one.name.to_lowercase().cmp(&other.name.to_lowercase());
        match sort {
            KeywordSort::Name => by_name,
            KeywordSort::Count => other.count.cmp(&one.count).then(by_name),
        }
    });
    for entry in here {
        rows.push(KeywordRow {
            name: entry.name.clone(),
            leaf: entry.name.rsplit('/').next().unwrap_or(&entry.name).to_string(),
            depth,
            count: entry.count,
            has_children: entries.iter().any(|other| in_parent(other, Some(&entry.name))),
        });
        push_keyword_level(entries, Some(&entry.name), depth + 1, sort, rows);
    }
}

/// The longest keyword kept, in characters.
const KEYWORD_MAX: usize = 40;

fn same_word(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// Whether two keywords are the same one, ignoring case in any alphabet.
pub fn same_keyword(one: &str, other: &str) -> bool {
    same_word(one, other)
}

/// Whether choosing `wanted` takes in an item carrying `word`: the same
/// keyword, or one nested under it with `/`. `plot` covers `plot/subplot`;
/// `subplot` does not, and neither covers `plotting`.
pub fn keyword_covers(wanted: &str, word: &str) -> bool {
    let wanted = wanted.trim().to_lowercase();
    let word = word.trim().to_lowercase();
    word == wanted || word.strip_prefix(&wanted).is_some_and(|rest| rest.starts_with('/'))
}

/// A keyword as it is kept, from what was typed: without a leading `#`,
/// without spaces around `/`, and at most `KEYWORD_MAX` characters. `None`
/// when it is empty or has an empty part (`a//b`, a `/` at either end).
pub fn tidy_keyword(typed: &str) -> Option<String> {
    let typed = typed.trim().trim_start_matches('#');
    let parts: Vec<&str> = typed.split('/').map(str::trim).collect();
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    let joined: String = parts.join("/").chars().take(KEYWORD_MAX).collect();
    let kept = joined.trim_end_matches(['/', ' ']).to_string();
    (!kept.is_empty()).then_some(kept)
}

fn keyword_colors() -> Vec<&'static str> {
    BINDER_COLORS.iter().map(|color| color.id).filter(|id| !id.is_empty()).collect()
}

impl BinderTree {
    /// The color of a keyword, or `""` when it has none.
    pub fn keyword_color(&self, name: &str) -> &str {
        self.keywords
            .iter()
            .find(|keyword| same_word(&keyword.name, name))
            .map(|keyword| keyword.color.as_str())
            .unwrap_or("")
    }

    /// Give an item a keyword the writer typed, tidied first
    /// (`tidy_keyword`). A keyword new to the project takes the next color
    /// in turn. Returns whether anything changed.
    pub fn add_keyword(&mut self, id: Id, name: &str) -> bool {
        match tidy_keyword(name) {
            Some(name) => self.carry_keyword(id, &name),
            None => false,
        }
    }

    /// Give an item a keyword that is already kept somewhere (another
    /// item's, or an older label), as it is spelled: what a project holds
    /// is not tidied again. Returns whether anything changed.
    pub fn carry_keyword(&mut self, id: Id, name: &str) -> bool {
        let name: String = name.trim().chars().take(KEYWORD_MAX).collect();
        if name.is_empty() {
            return false;
        }
        let Some(node) = self.get(id) else {
            return false;
        };
        if node.keywords.iter().any(|kept| same_word(kept, &name)) {
            return false;
        }
        // A keyword already in use keeps the spelling it was first given.
        let spelled = self
            .keywords
            .iter()
            .find(|keyword| same_word(&keyword.name, &name))
            .map(|keyword| keyword.name.clone());
        let name = spelled.clone().unwrap_or(name);
        if let Some(node) = self.get_mut(id) {
            node.keywords.push(name.clone());
            node.updated_at = Timestamp::now();
        }
        if spelled.is_none() {
            let colors = keyword_colors();
            let color = colors[self.keywords.len() % colors.len()].to_string();
            self.keywords.push(Keyword { name, color });
        }
        true
    }

    /// Take a keyword off an item. A keyword no item carries any more is
    /// forgotten, with its color.
    pub fn remove_keyword(&mut self, id: Id, name: &str) -> bool {
        let Some(node) = self.get_mut(id) else {
            return false;
        };
        let before = node.keywords.len();
        node.keywords.retain(|kept| !same_word(kept, name));
        if node.keywords.len() == before {
            return false;
        }
        node.updated_at = Timestamp::now();
        let still_used = self
            .nodes
            .iter()
            .any(|node| node.keywords.iter().any(|kept| same_word(kept, name)));
        if !still_used {
            self.keywords.retain(|keyword| !same_word(&keyword.name, name));
        }
        true
    }

    /// Labels became keywords: each item's label is added to its keywords
    /// and cleared. A project from before keywords opens with its labels
    /// kept this way. Returns whether anything changed.
    pub fn fold_labels_into_keywords(&mut self) -> bool {
        let labelled: Vec<(Id, String)> = self
            .nodes
            .iter()
            .filter(|node| !node.label.trim().is_empty())
            .map(|node| (node.id, node.label.clone()))
            .collect();
        let mut changed = false;
        for (id, label) in labelled {
            self.carry_keyword(id, &label);
            if let Some(node) = self.get_mut(id) {
                node.label.clear();
            }
            changed = true;
        }
        changed
    }

    /// The keywords a menu offers: each one in use, each after the parents
    /// it is nested under, so a parent no item carries itself can still be
    /// chosen. Each once, spelled as it first comes.
    pub fn keyword_choices(&self) -> Vec<String> {
        let mut choices: Vec<String> = Vec::new();
        for keyword in &self.keywords {
            let name = keyword.name.as_str();
            let nested = name.match_indices('/').map(|(at, _)| &name[..at]);
            for word in nested.chain(std::iter::once(name)) {
                if !choices.iter().any(|kept| same_word(kept, word)) {
                    choices.push(word.to_string());
                }
            }
        }
        choices
    }

    /// How many items carry a keyword or one nested under it, the trash
    /// included: the items a rename changes.
    pub fn items_with_keyword(&self, name: &str) -> usize {
        self.nodes.iter().filter(|node| node.keywords.iter().any(|word| keyword_covers(name, word))).count()
    }

    /// Whether renaming `old` to `new` would fold it into a keyword already
    /// in use, rather than give it a new name or a new spelling.
    pub fn keyword_merges(&self, old: &str, new: &str) -> bool {
        let Some(new) = tidy_keyword(new) else {
            return false;
        };
        self.keywords.iter().any(|keyword| same_word(&keyword.name, &new) && !keyword_covers(old, &keyword.name))
    }

    /// Rename a keyword on every item and saved filter, with what is nested
    /// under it: `plot` to `story` makes `plot/subplot` into
    /// `story/subplot`. A name already in use merges the two, keeping that
    /// keyword's spelling and color. Returns how many items changed, or why
    /// the new name cannot be taken.
    pub fn rename_keyword(&mut self, old: &str, new: &str) -> std::result::Result<usize, String> {
        let new = tidy_keyword(new).ok_or_else(|| format!("“{}” is not a keyword: each part around / needs a name.", new.trim()))?;
        let depth = old.trim().split('/').count();
        // The keywords that stay as they are, which a renamed one may join.
        let staying: Vec<String> = self
            .keywords
            .iter()
            .filter(|keyword| !keyword_covers(old, &keyword.name))
            .map(|keyword| keyword.name.clone())
            .collect();
        // Joining a keyword in use, its children take its spelling too.
        let new = staying.iter().find(|kept| same_word(kept, &new)).cloned().unwrap_or(new);
        let moved = |word: &str| -> Option<String> {
            if !keyword_covers(old, word) {
                return None;
            }
            let rest: Vec<&str> = word.trim().split('/').skip(depth).collect();
            let name = if rest.is_empty() { new.clone() } else { format!("{new}/{}", rest.join("/")) };
            Some(staying.iter().find(|kept| same_word(kept, &name)).cloned().unwrap_or(name))
        };
        let renamed = self
            .nodes
            .iter()
            .flat_map(|node| node.keywords.iter())
            .chain(self.keywords.iter().map(|keyword| &keyword.name))
            .chain(self.filters.iter().flat_map(|filter| filter.keywords.iter().chain(&filter.without)))
            .filter_map(|word| moved(word));
        for name in renamed {
            if name.chars().count() > KEYWORD_MAX {
                return Err(format!("“{name}” would be longer than {KEYWORD_MAX} characters."));
            }
        }

        let mut changed = 0;
        for node in &mut self.nodes {
            if !node.keywords.iter().any(|word| keyword_covers(old, word)) {
                continue;
            }
            let mut words: Vec<String> = Vec::new();
            for word in &node.keywords {
                let word = moved(word).unwrap_or_else(|| word.clone());
                if !words.iter().any(|kept| same_word(kept, &word)) {
                    words.push(word);
                }
            }
            node.keywords = words;
            node.updated_at = Timestamp::now();
            changed += 1;
        }
        let mut keywords: Vec<Keyword> = Vec::new();
        for keyword in &self.keywords {
            match moved(&keyword.name) {
                // A keyword that stays keeps its place and its color.
                Some(name) if staying.iter().any(|kept| same_word(kept, &name)) => {}
                Some(name) if keywords.iter().any(|kept| same_word(&kept.name, &name)) => {}
                Some(name) => keywords.push(Keyword { name, color: keyword.color.clone() }),
                None => keywords.push(keyword.clone()),
            }
        }
        self.keywords = keywords;
        for filter in &mut self.filters {
            for word in filter.keywords.iter_mut().chain(filter.without.iter_mut()) {
                if let Some(name) = moved(word) {
                    *word = name;
                }
            }
        }
        Ok(changed)
    }

    /// Every keyword as a tree, for the library's list: each under the one
    /// it is nested in, siblings sorted by `sort`, each counting the items
    /// outside the trash that carry it or one nested under it.
    pub fn keyword_tree(&self, sort: KeywordSort) -> Vec<KeywordRow> {
        let kept: Vec<&BinderNode> = self
            .nodes
            .iter()
            .filter(|node| !node.keywords.is_empty() && node.deleted_at.is_none() && !self.is_in_trash(node.id))
            .collect();
        let entries: Vec<KeywordEntry> = self
            .keyword_choices()
            .into_iter()
            .map(|name| KeywordEntry {
                parent: name.rfind('/').map(|at| name[..at].to_string()),
                count: kept.iter().filter(|node| node.keywords.iter().any(|word| keyword_covers(&name, word))).count(),
                name,
            })
            .collect();
        let mut rows = Vec::new();
        push_keyword_level(&entries, None, 0, sort, &mut rows);
        rows
    }

    /// Give a keyword the next binder color, for every item that carries it.
    pub fn cycle_keyword_color(&mut self, name: &str) -> bool {
        let colors = keyword_colors();
        let Some(keyword) = self.keywords.iter_mut().find(|keyword| same_word(&keyword.name, name)) else {
            return false;
        };
        let next = colors
            .iter()
            .position(|color| *color == keyword.color)
            .map(|at| at + 1)
            .unwrap_or(0);
        keyword.color = colors[next % colors.len()].to_string();
        true
    }

    /// The keywords an item has, with those of the chapter, part, or book
    /// it is in: a chapter's keyword counts for every scene in it.
    pub fn keywords_through(&self, id: Id) -> Vec<&str> {
        self.chain_of(id).into_iter().flat_map(|node| node.keywords.iter().map(String::as_str)).collect()
    }

    /// Whether an item, through what it is in, carries the keywords asked
    /// for (all of them, or one with `any`) and none of `without`; each
    /// takes in what is nested under it.
    pub fn keywords_match(&self, id: Id, with: &[String], any: bool, without: &[String]) -> bool {
        let words = self.keywords_through(id);
        let carries = |wanted: &String| words.iter().any(|word| keyword_covers(wanted, word));
        let wanted = with.is_empty() || if any { with.iter().any(carries) } else { with.iter().all(carries) };
        wanted && !without.iter().any(carries)
    }

    /// The manuscript's scenes, in binder order, that match the filter: its
    /// keywords on the scene or the chapter, part, or book it is in, and its
    /// status on the scene itself (a chapter's status is no scene's). The
    /// text is matched by the caller, which has the search index.
    pub fn scenes_matching(&self, filter: &SavedFilter) -> Vec<Id> {
        let Some(draft) = self.draft_root() else {
            return Vec::new();
        };
        let status = filter.status.trim();
        self.scenes_under(draft.id)
            .into_iter()
            .map(|(_, scene)| scene)
            .filter(|scene| self.keywords_match(*scene, &filter.keywords, filter.any, &filter.without))
            .filter(|scene| status.is_empty() || self.get(*scene).is_some_and(|node| same_word(&node.status, status)))
            .collect()
    }

    /// An item and what it is in, up to the top.
    fn chain_of(&self, id: Id) -> Vec<&BinderNode> {
        let mut chain = Vec::new();
        let mut at = Some(id);
        while let Some(node) = at.and_then(|id| self.get(id)) {
            chain.push(node);
            at = node.parent_id;
            if chain.len() > 10_000 {
                break;
            }
        }
        chain
    }
}

#[cfg(test)]
mod keyword_tests {
    use super::*;

    fn tree() -> (BinderTree, Id, Id) {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter = BinderNode::new(NodeKind::Chapter, "Chapter", Some(book.id), 0);
        let a = BinderNode::new(NodeKind::Scene, "A", Some(chapter.id), 0);
        let b = BinderNode::new(NodeKind::Scene, "B", Some(chapter.id), 1);
        let (a_id, b_id) = (a.id, b.id);
        (BinderTree::new(vec![draft, book, chapter, a, b]), a_id, b_id)
    }

    #[test]
    fn a_keyword_has_one_color_and_is_forgotten_when_no_item_carries_it() {
        let (mut tree, a, b) = tree();
        assert!(tree.add_keyword(a, " Flashback "));
        assert!(!tree.add_keyword(a, "flashback"), "the same word is not added twice");
        assert!(tree.add_keyword(b, "FLASHBACK"));
        assert_eq!(tree.get(b).unwrap().keywords, ["Flashback"], "it keeps its first spelling");
        assert_eq!(tree.keywords.len(), 1);
        let first = tree.keyword_color("flashback").to_string();
        assert!(!first.is_empty());
        assert!(tree.add_keyword(a, "Mara"));
        assert_ne!(tree.keyword_color("Mara"), first, "a new keyword takes the next color");
        tree.cycle_keyword_color("Flashback");
        assert_ne!(tree.keyword_color("Flashback"), first);
        tree.remove_keyword(a, "flashback");
        assert_eq!(tree.keywords.len(), 2, "b still carries it");
        tree.remove_keyword(b, "Flashback");
        assert_eq!(tree.keyword_color("Flashback"), "");
        assert!(!tree.add_keyword(a, "   "));
    }

    #[test]
    fn a_filter_matches_scenes_by_keyword_status_and_label() {
        let (mut tree, a, b) = tree();
        tree.add_keyword(a, "Flashback");
        tree.get_mut(b).unwrap().status = "Revise".into();
        let filter = |keyword: &str, status: &str| SavedFilter {
            status: status.into(),
            ..keyword_filter(keyword)
        };
        assert_eq!(tree.scenes_matching(&filter("flashback", "")), [a]);
        assert_eq!(tree.scenes_matching(&filter("", "revise")), [b]);
        assert_eq!(tree.scenes_matching(&filter("", "")), [a, b]);
        assert!(tree.scenes_matching(&filter("Flashback", "Revise")).is_empty());
        // Status is the scene's own: its chapter, a draft too, does not
        // bring a scene under revision into "draft".
        assert_eq!(tree.scenes_matching(&filter("", "draft")), [a]);
    }

    #[test]
    fn labels_become_keywords() {
        let (mut tree, a, b) = tree();
        tree.get_mut(a).unwrap().label = "subplot".into();
        tree.get_mut(b).unwrap().label = "Subplot".into();
        tree.add_keyword(b, "Mara");
        assert!(tree.fold_labels_into_keywords());
        assert_eq!(tree.get(a).unwrap().keywords, ["subplot"]);
        assert_eq!(tree.get(b).unwrap().keywords, ["Mara", "subplot"], "one keyword for both spellings");
        assert!(tree.nodes.iter().all(|node| node.label.is_empty()));
        assert!(!tree.fold_labels_into_keywords(), "nothing left to fold");
    }

    #[test]
    fn a_chapter_with_the_keyword_brings_its_scenes_into_the_filter() {
        let (mut tree, a, b) = tree();
        let chapter = tree.get(a).unwrap().parent_id.unwrap();
        tree.add_keyword(chapter, "Act two");
        assert_eq!(tree.scenes_matching(&keyword_filter("act two")), [a, b]);
    }

    fn keyword_filter(keyword: &str) -> SavedFilter {
        SavedFilter {
            id: Id::new(),
            name: keyword.into(),
            keywords: if keyword.is_empty() { Vec::new() } else { vec![keyword.into()] },
            any: false,
            without: Vec::new(),
            entries: Vec::new(),
            status: String::new(),
            text: String::new(),
        }
    }

    /// A filter takes several keywords, all of them or any, and keywords to
    /// leave out; a chapter's keyword counts with its scene's.
    #[test]
    fn a_filter_combines_keywords_all_any_and_without() {
        let (mut tree, a, b) = tree();
        let chapter = tree.get(a).unwrap().parent_id.unwrap();
        tree.add_keyword(chapter, "act two");
        tree.add_keyword(a, "flashback");
        tree.add_keyword(b, "Mara/young");
        let filter = |with: &[&str], any: bool, without: &[&str]| SavedFilter {
            keywords: with.iter().map(|word| word.to_string()).collect(),
            any,
            without: without.iter().map(|word| word.to_string()).collect(),
            ..keyword_filter("")
        };
        assert_eq!(tree.scenes_matching(&filter(&["act two", "flashback"], false, &[])), [a], "the chapter's and the scene's");
        assert!(tree.scenes_matching(&filter(&["flashback", "mara"], false, &[])).is_empty());
        assert_eq!(tree.scenes_matching(&filter(&["flashback", "mara"], true, &[])), [a, b]);
        assert_eq!(tree.scenes_matching(&filter(&["act two"], false, &["mara"])), [a], "mara takes in Mara/young");
        assert!(tree.scenes_matching(&filter(&[], false, &["act two"])).is_empty(), "left out through the chapter");

        // A rename reaches the keywords a filter leaves out too.
        tree.filters.push(filter(&["flashback"], false, &["mara/young"]));
        tree.rename_keyword("mara", "Maren").unwrap();
        assert_eq!(tree.filters[0].without, ["Maren/young"]);
    }

    #[test]
    fn a_search_is_split_into_keywords_left_out_keywords_and_words() {
        let query = KeywordQuery::parse("  #plot / harbor -#Draft  #  the  -#a//b night ");
        assert_eq!(query.with, ["plot"]);
        assert_eq!(query.without, ["Draft"]);
        assert_eq!(query.text, "/ harbor # the -#a//b night", "a # with no keyword is a word");
        assert_eq!(KeywordQuery::parse("harbor").text, "harbor");
        assert!(KeywordQuery::parse("#plot").text.is_empty());
    }

    /// `plot` covers `plot/subplot`, as a parent tag covers its children in
    /// Obsidian; never the other way, and never by a part of the name.
    #[test]
    fn a_parent_keyword_brings_its_children_and_not_the_other_way() {
        let (mut tree, a, b) = tree();
        tree.add_keyword(a, "plot/subplot");
        tree.add_keyword(b, "plotting");
        assert_eq!(tree.scenes_matching(&keyword_filter("Plot")), [a]);
        assert_eq!(tree.scenes_matching(&keyword_filter("plot/subplot")), [a]);
        assert!(tree.scenes_matching(&keyword_filter("subplot")).is_empty());
        assert!(tree.scenes_matching(&keyword_filter("plot/subplot/twist")).is_empty());
        assert!(keyword_covers("Ação", "AÇÃO/fuga"), "case is ignored outside ASCII too");
        assert!(same_keyword("Ação", "AÇÃO"));
        assert!(!keyword_covers("plot", "plotting"));
    }

    #[test]
    fn a_keyword_is_tidied_when_it_is_entered() {
        assert_eq!(tidy_keyword(" #plot / subplot ").as_deref(), Some("plot/subplot"));
        assert_eq!(tidy_keyword("##Mara").as_deref(), Some("Mara"));
        assert_eq!(tidy_keyword("act two").as_deref(), Some("act two"), "spaces inside a part stay");
        for broken in ["", "  ", "#", "a//b", "/a", "a/", "a/ /b"] {
            assert_eq!(tidy_keyword(broken), None, "{broken:?}");
        }
        let long = format!("{}/{}", "a".repeat(39), "b".repeat(10));
        let kept = tidy_keyword(&long).unwrap();
        assert_eq!(kept.chars().count(), KEYWORD_MAX - 1, "cut where it does not end on a /");
        assert!(!kept.ends_with('/'));

        let (mut tree, a, _) = tree();
        assert!(tree.add_keyword(a, "#plot / subplot"));
        assert_eq!(tree.get(a).unwrap().keywords, ["plot/subplot"]);
        assert!(!tree.add_keyword(a, "a//b"));
        // What a project already holds is carried as it is.
        assert!(tree.carry_keyword(a, "#todo/"));
        assert_eq!(tree.get(a).unwrap().keywords, ["plot/subplot", "#todo/"]);
        tree.get_mut(a).unwrap().label = "later/".into();
        tree.fold_labels_into_keywords();
        assert!(tree.get(a).unwrap().keywords.contains(&"later/".to_string()), "an old label is not lost");
    }

    /// The keyword menus offer a parent that no item carries itself, so it
    /// can be chosen to bring all its children.
    #[test]
    fn the_keyword_choices_add_each_parent_once_before_its_children() {
        let (mut tree, a, b) = tree();
        tree.add_keyword(a, "Plot/Subplot");
        tree.add_keyword(b, "flashback");
        tree.add_keyword(b, "plot/twist/late");
        assert_eq!(
            tree.keyword_choices(),
            ["Plot", "Plot/Subplot", "flashback", "plot/twist", "plot/twist/late"],
        );
        tree.add_keyword(a, "plot");
        assert_eq!(tree.keyword_choices().iter().filter(|word| same_keyword(word, "plot")).count(), 1);
    }

    /// Renaming a keyword renames it on every item, its children with it,
    /// and in the saved filters; the color stays with it.
    #[test]
    fn a_renamed_keyword_takes_its_children_and_filters_along() {
        let (mut tree, a, b) = tree();
        tree.add_keyword(a, "Plot");
        tree.add_keyword(a, "plot/subplot");
        tree.add_keyword(b, "plot/twist/late");
        tree.add_keyword(b, "plotting");
        tree.filters.push(keyword_filter("plot/twist"));
        let color = tree.keyword_color("plot/subplot").to_string();
        assert_eq!(tree.items_with_keyword("PLOT"), 2);
        assert!(!tree.keyword_merges("plot", "story"));

        assert_eq!(tree.rename_keyword("plot", " #Story "), Ok(2));
        assert_eq!(tree.get(a).unwrap().keywords, ["Story", "Story/subplot"]);
        assert_eq!(tree.get(b).unwrap().keywords, ["Story/twist/late", "plotting"], "plotting is another keyword");
        assert_eq!(tree.filters[0].keywords, ["Story/twist"]);
        assert_eq!(tree.keyword_color("story/subplot"), color, "the color goes with it");
        assert!(tree.keywords.iter().all(|keyword| !keyword_covers("plot", &keyword.name)));

        assert!(tree.rename_keyword("story", "a//b").is_err(), "the new name is tidied like a typed one");
        assert_eq!(tree.rename_keyword("nothing", "else"), Ok(0));
        assert_eq!(tree.rename_keyword("story", "STORY"), Ok(2), "a new spelling of the same keyword");
        assert_eq!(tree.get(a).unwrap().keywords, ["STORY", "STORY/subplot"]);
    }

    /// Renaming to a keyword in use merges the two: an item carrying both
    /// keeps one, spelled and colored as the one that was there.
    #[test]
    fn renaming_to_a_keyword_in_use_merges_them() {
        let (mut tree, a, b) = tree();
        tree.add_keyword(a, "Mara");
        tree.add_keyword(b, "Mara");
        tree.add_keyword(b, "mara-old");
        tree.add_keyword(a, "mara-old/scar");
        tree.add_keyword(b, "Mara/scar");
        tree.add_keyword(a, "mara-old/eyes");
        let kept_color = tree.keyword_color("Mara").to_string();
        assert!(tree.keyword_merges("mara-old", "mara"));
        assert!(!tree.keyword_merges("mara", "MARA"), "a new spelling is not a merge");

        assert_eq!(tree.rename_keyword("mara-old", "mara"), Ok(2));
        assert_eq!(tree.get(a).unwrap().keywords, ["Mara", "Mara/scar", "Mara/eyes"], "a new child takes the kept spelling");
        assert_eq!(tree.get(b).unwrap().keywords, ["Mara", "Mara/scar"], "one of each, not two");
        assert_eq!(tree.keyword_color("mara"), kept_color);
        let names: Vec<&str> = tree.keywords.iter().map(|keyword| keyword.name.as_str()).collect();
        assert_eq!(names, ["Mara", "Mara/scar", "Mara/eyes"]);
    }

    /// The library's keyword list: a tree, each keyword counting the items
    /// that carry it or one nested under it, the trash left out.
    #[test]
    fn the_keyword_tree_nests_counts_and_sorts() {
        let (mut tree, a, b) = tree();
        let chapter = tree.get(a).unwrap().parent_id.unwrap();
        tree.add_keyword(a, "plot/subplot");
        tree.add_keyword(a, "plot");
        tree.add_keyword(b, "plot/twist");
        tree.add_keyword(chapter, "Flashback");
        tree.add_keyword(b, "flashback");
        let rows = |tree: &BinderTree, sort: KeywordSort| -> Vec<(String, String, usize, usize, bool)> {
            tree.keyword_tree(sort)
                .into_iter()
                .map(|row| (row.name, row.leaf, row.depth, row.count, row.has_children))
                .collect()
        };
        let row = |name: &str, leaf: &str, depth: usize, count: usize, children: bool| {
            (name.to_string(), leaf.to_string(), depth, count, children)
        };
        assert_eq!(
            rows(&tree, KeywordSort::Name),
            [
                row("Flashback", "Flashback", 0, 2, false),
                row("plot", "plot", 0, 2, true),
                row("plot/subplot", "subplot", 1, 1, false),
                row("plot/twist", "twist", 1, 1, false),
            ],
            "a item carrying plot and plot/subplot counts once for plot",
        );

        // A scene in the trash is not counted, and a keyword only it carries
        // still shows, at nothing.
        let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 9);
        let trash_id = trash.id;
        tree.nodes.push(trash);
        tree.get_mut(b).unwrap().parent_id = Some(trash_id);
        tree.add_keyword(a, "act one");
        tree.add_keyword(chapter, "act one");
        let by_count = rows(&tree, KeywordSort::Count);
        assert_eq!(
            by_count,
            [
                row("act one", "act one", 0, 2, false),
                row("Flashback", "Flashback", 0, 1, false),
                row("plot", "plot", 0, 1, true),
                row("plot/subplot", "subplot", 1, 1, false),
                row("plot/twist", "twist", 1, 0, false),
            ],
            "most used first, then by name, each level apart",
        );
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    #[test]
    fn the_next_scene_is_found_across_chapters_and_put_back_after_the_trash() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 1);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "One", Some(book.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Two", Some(book.id), 1);
        let a = BinderNode::new(NodeKind::Scene, "A", Some(one.id), 0);
        let b = BinderNode::new(NodeKind::Scene, "B", Some(one.id), 1);
        let c = BinderNode::new(NodeKind::Scene, "C", Some(one.id), 2);
        let d = BinderNode::new(NodeKind::Scene, "D", Some(two.id), 0);
        let (one_id, a_id, b_id, c_id, d_id) = (one.id, a.id, b.id, c.id, d.id);
        let mut tree = BinderTree::new(vec![draft, trash, book, one, two, a, b, c, d]);
        assert_eq!(tree.scene_after(a_id), Some(b_id));
        assert_eq!(tree.scene_after(c_id), Some(d_id), "the next chapter's first scene");
        assert_eq!(tree.scene_after(d_id), None);
        assert_eq!(tree.previous_sibling(b_id), Some(a_id));
        assert_eq!(tree.previous_sibling(a_id), None);

        tree.trash(b_id).unwrap();
        assert_eq!(tree.scene_after(a_id), Some(c_id));
        tree.put_after(b_id, one_id, Some(a_id)).unwrap();
        let order: Vec<Id> = tree.children(Some(one_id)).iter().map(|node| node.id).collect();
        assert_eq!(order, [a_id, b_id, c_id]);
        assert!(!tree.is_in_trash(b_id));
        assert!(tree.get(b_id).unwrap().deleted_at.is_none());
    }
}

#[cfg(test)]
mod left_out_tests {
    use super::*;

    #[test]
    fn a_scene_is_left_out_by_itself_or_by_what_it_is_in() {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let mut chapter = BinderNode::new(NodeKind::Chapter, "Chapter", Some(book.id), 0);
        let scene = BinderNode::new(NodeKind::Scene, "Scene", Some(chapter.id), 0);
        let (chapter_id, scene_id) = (chapter.id, scene.id);
        chapter.left_out = true;
        let mut tree = BinderTree::new(vec![draft, book, chapter, scene]);
        assert!(tree.is_left_out(scene_id));
        tree.get_mut(chapter_id).unwrap().left_out = false;
        assert!(!tree.is_left_out(scene_id));
    }
}
