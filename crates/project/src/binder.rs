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
    pub deleted_at: Option<Timestamp>,
    pub restore_parent_id: Option<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl BinderNode {
    pub fn new(kind: NodeKind, title: impl Into<String>, parent_id: Option<Id>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        let kind_copy = kind;
        Self {
            id: Id::new(),
            parent_id,
            kind,
            title: title.into(),
            sort_index,
            document_id: if kind_copy.can_hold_document() {
                Some(Id::new())
            } else {
                None
            },
            synopsis: String::new(),
            label: String::new(),
            status: String::from("draft"),
            color: String::new(),
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
}

impl BinderTree {
    pub fn new(nodes: Vec<BinderNode>) -> Self {
        Self { nodes }
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
