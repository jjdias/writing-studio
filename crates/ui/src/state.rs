use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use writing_studio_core::{autosave_due, autosave_wait, Id, PRODUCT_NAME};
use writing_studio_editor::EditorSession;
use writing_studio_project::{BinderNode, BinderTree, NodeKind, ProjectMeta};
use writing_studio_storage::{BackupInfo, ProjectStore, SnapshotInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceMode {
    Writing,
    Corkboard,
    Outliner,
    History,
}

impl WorkspaceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Writing => "Writing",
            Self::Corkboard => "Corkboard",
            Self::Outliner => "Outliner",
            Self::History => "History",
        }
    }
}

#[derive(Clone)]
pub struct StudioState {
    pub product_name: &'static str,
    pub store: Option<Rc<RefCell<ProjectStore>>>,
    pub meta: Option<ProjectMeta>,
    pub binder: BinderTree,
    pub selected: Option<Id>,
    pub session: Option<EditorSession>,
    pub editor_text: String,
    pub search: String,
    pub mode: WorkspaceMode,
    pub focus_mode: bool,
    pub status: String,
    pub new_title: String,
    pub confirm_backup: Option<PathBuf>,
    pub binder_dirty: bool,
    pub last_edit: Option<Instant>,
    pub dragging: Option<Id>,
    pub drop_target: Option<Id>,
    pub confirm_empty_trash: bool,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            product_name: PRODUCT_NAME,
            store: None,
            meta: None,
            binder: BinderTree::new(Vec::new()),
            selected: None,
            session: None,
            editor_text: String::new(),
            search: String::new(),
            mode: WorkspaceMode::Writing,
            focus_mode: false,
            status: "Offline · Local-first".into(),
            new_title: "Untitled Novel".into(),
            confirm_backup: None,
            binder_dirty: false,
            last_edit: None,
            dragging: None,
            drop_target: None,
            confirm_empty_trash: false,
        }
    }
}

impl StudioState {
    pub fn is_open(&self) -> bool {
        self.store.is_some()
    }

    pub fn project_title(&self) -> String {
        self.meta
            .as_ref()
            .map(|meta| meta.title.clone())
            .unwrap_or_else(|| "No project".into())
    }

    pub fn word_count(&self) -> u32 {
        self.session.as_ref().map(EditorSession::word_count).unwrap_or(0)
    }

    pub fn project_word_count(&self) -> u32 {
        let store = match &self.store {
            Some(store) => store.borrow(),
            None => return 0,
        };
        self.binder.total_word_count(|id| store.document_word_count(id))
    }

    pub fn goal(&self) -> u32 {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.project_word_goal)
            .unwrap_or(80_000)
    }

    pub fn selected_node(&self) -> Option<&BinderNode> {
        self.selected.and_then(|id| self.binder.get(id))
    }

    pub fn create_project(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a folder for the new project")
            .pick_folder()
        else {
            return;
        };
        let title = self.new_title.trim();
        let title = if title.is_empty() { "Untitled Novel" } else { title };
        let root: PathBuf = folder.join(format!("{}.writingstudio", sanitize(title)));
        match ProjectStore::create(&root, title) {
            Ok(store) => self.attach_store(store, "Created project and saved locally."),
            Err(err) => self.status = format!("Could not create project: {err}"),
        }
    }

    pub fn open_project(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Open a Writing Studio project folder")
            .pick_folder()
        else {
            return;
        };
        match ProjectStore::open(&folder) {
            Ok(store) => self.attach_store(store, "Opened local project."),
            Err(err) => self.status = format!("Could not open project: {err}"),
        }
    }

    fn attach_store(&mut self, store: ProjectStore, status: &str) {
        match (store.load_meta(), store.load_binder()) {
            (Ok(meta), Ok(binder)) => {
                self.meta = Some(meta);
                self.binder = binder;
                self.store = Some(Rc::new(RefCell::new(store)));
                self.status = status.into();
                if let Some(scene) = self
                    .binder
                    .nodes
                    .iter()
                    .find(|node| node.kind == NodeKind::Scene)
                {
                    let id = scene.id;
                    self.select_node(id);
                }
            }
            (Err(err), _) | (_, Err(err)) => {
                self.status = format!("Project load failed: {err}");
            }
        }
    }

    pub fn select_node(&mut self, id: Id) {
        self.flush_editor();
        self.selected = Some(id);
        let document_id = self.binder.get(id).and_then(|node| node.document_id);
        if let (Some(store), Some(doc_id)) = (&self.store, document_id) {
            match store.borrow().load_or_create_document(doc_id) {
                Ok(document) => {
                    let session = EditorSession::new(document);
                    self.editor_text = session.plain_text();
                    self.session = Some(session);
                }
                Err(err) => self.status = format!("Could not load document: {err}"),
            }
        } else {
            self.session = None;
            self.editor_text.clear();
        }
    }

    pub fn apply_editor_text(&mut self, text: String) {
        self.editor_text = text.clone();
        if let Some(session) = &mut self.session {
            session.apply_plain_text(&text);
            if session.is_dirty() {
                self.note_edit();
            }
        }
    }

    pub fn flush_editor(&mut self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        if let Some(session) = &mut self.session {
            if !session.is_dirty() {
                return;
            }
            match store.borrow().save_document(session.document()) {
                Ok(()) => {
                    session.mark_clean();
                    self.status = "Saved".into();
                }
                Err(err) => self.status = format!("Save failed: {err}"),
            }
        }
    }

    pub fn save_all(&mut self) {
        self.flush_editor();
        if let Some(node) = self.selected.and_then(|id| self.binder.get(id).cloned()) {
            if let Some(store) = &self.store {
                if let Err(err) = store.borrow().upsert_node(&node) {
                    self.status = format!("Could not save binder: {err}");
                    return;
                }
            }
        }
        if let Some(meta) = &self.meta {
            if let Some(store) = &self.store {
                if let Err(err) = store.borrow().persist_meta(meta) {
                    self.status = format!("Could not save project: {err}");
                    return;
                }
            }
        }
        if self.session.as_ref().is_some_and(EditorSession::is_dirty) {
            return;
        }
        self.binder_dirty = false;
        self.last_edit = None;
        self.status = "All changes saved locally.".into();
    }

    pub fn autosave_tick(&mut self) {
        if !self.is_open() {
            return;
        }
        let wait = autosave_wait(
            self.meta
                .as_ref()
                .map(|meta| meta.settings.autosave_seconds)
                .unwrap_or(2),
        );
        if !autosave_due(self.has_unsaved(), self.last_edit, Instant::now(), wait) {
            return;
        }
        self.save_all();
        if self.has_unsaved() {
            return;
        }
        self.status = "Autosaved locally.".into();
    }

    pub fn has_unsaved(&self) -> bool {
        self.binder_dirty || self.session.as_ref().is_some_and(EditorSession::is_dirty)
    }

    fn note_edit(&mut self) {
        self.last_edit = Some(Instant::now());
    }

    pub fn backup(&mut self) {
        self.save_all();
        if let Some(store) = &self.store {
            match store.borrow().backup() {
                Ok(path) => self.status = format!("Backup written to {}", path.display()),
                Err(err) => self.status = format!("Backup failed: {err}"),
            }
        }
    }

    pub fn snapshot(&mut self) {
        self.flush_editor();
        if let (Some(store), Some(session)) = (&self.store, &self.session) {
            match store.borrow().snapshot_document(session.document(), "manual") {
                Ok(_) => self.status = "Snapshot saved locally.".into(),
                Err(err) => self.status = format!("Snapshot failed: {err}"),
            }
        }
    }

    pub fn snapshots_for_selected(&self) -> Vec<SnapshotInfo> {
        let Some(doc_id) = self.selected_node().and_then(|node| node.document_id) else {
            return Vec::new();
        };
        self.store
            .as_ref()
            .and_then(|store| store.borrow().list_snapshots(doc_id).ok())
            .unwrap_or_default()
    }

    pub fn backups(&self) -> Vec<BackupInfo> {
        self.store
            .as_ref()
            .and_then(|store| store.borrow().list_backups().ok())
            .unwrap_or_default()
    }

    pub fn restore_snapshot(&mut self, snapshot_id: Id) {
        self.flush_editor();
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().restore_snapshot(snapshot_id) {
            Ok(document) => {
                let session = EditorSession::new(document);
                self.editor_text = session.plain_text();
                self.session = Some(session);
                self.status =
                    "Snapshot restored locally. Previous text was saved as a pre-restore snapshot."
                        .into();
            }
            Err(err) => self.status = format!("Could not restore snapshot: {err}"),
        }
    }

    pub fn request_backup_restore(&mut self, path: PathBuf) {
        if self.confirm_backup.as_ref() == Some(&path) {
            self.restore_backup_file(path);
        } else {
            self.confirm_backup = Some(path);
            self.status =
                "Click restore again to replace the live project with this local backup. A safety copy is written first."
                    .into();
        }
    }

    fn restore_backup_file(&mut self, path: PathBuf) {
        self.confirm_backup = None;
        self.flush_editor();
        let Some(store) = self.store.clone() else {
            return;
        };
        let restored = store.borrow_mut().restore_backup(&path);
        match restored {
            Ok(()) => self.reload_after_backup(
                "Restored local backup. A safety copy of the previous live project was saved first.",
            ),
            Err(err) => self.status = format!("Could not restore backup: {err}"),
        }
    }

    fn reload_after_backup(&mut self, status: &str) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let (meta, binder) = {
            let store = store.borrow();
            (store.load_meta(), store.load_binder())
        };
        match (meta, binder) {
            (Ok(meta), Ok(binder)) => {
                let previous = self.selected;
                self.meta = Some(meta);
                self.binder = binder;
                self.session = None;
                self.editor_text.clear();
                self.status = status.into();
                if let Some(id) = previous.filter(|id| self.binder.get(*id).is_some()) {
                    self.select_node(id);
                } else if let Some(scene) = self
                    .binder
                    .nodes
                    .iter()
                    .find(|node| node.kind == NodeKind::Scene)
                {
                    let id = scene.id;
                    self.select_node(id);
                }
            }
            (Err(err), _) | (_, Err(err)) => {
                self.status = format!("Backup restored, but reload failed: {err}");
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(session) = &mut self.session {
            if session.undo() {
                self.editor_text = session.plain_text();
                self.note_edit();
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(session) = &mut self.session {
            if session.redo() {
                self.editor_text = session.plain_text();
                self.note_edit();
            }
        }
    }

    pub fn add_node(&mut self, kind: NodeKind) {
        self.flush_editor();
        let parent_id = self.infer_parent(kind);
        let sort = self.binder.next_sort_index(parent_id);
        let title = match kind {
            NodeKind::Chapter => format!(
                "Chapter {}",
                self.binder
                    .nodes
                    .iter()
                    .filter(|node| node.kind == NodeKind::Chapter)
                    .count()
                    + 1
            ),
            NodeKind::Scene => format!(
                "Scene {}",
                self.binder
                    .nodes
                    .iter()
                    .filter(|node| node.kind == NodeKind::Scene)
                    .count()
                    + 1
            ),
            NodeKind::Note => "New note".into(),
            NodeKind::Folder => "New folder".into(),
            other => other.icon_label().to_string(),
        };
        let node = BinderNode::new(kind, title, parent_id, sort);
        let new_id = node.id;
        if let Some(store) = &self.store {
            if let Err(err) = store.borrow().upsert_node(&node) {
                self.status = format!("Could not add item: {err}");
                return;
            }
            if let Some(doc_id) = node.document_id {
                let document = writing_studio_document::Document::new(doc_id);
                if let Err(err) = store.borrow().save_document(&document) {
                    self.status = format!("Could not create document: {err}");
                    return;
                }
            }
        }
        self.binder.nodes.push(node);
        self.select_node(new_id);
        self.status = "Added to binder.".into();
    }

    fn persist_binder(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        if let Err(err) = store.borrow().persist_binder(&self.binder) {
            self.status = format!("Could not save binder: {err}");
            false
        } else {
            true
        }
    }

    pub fn begin_drag(&mut self, id: Id) {
        if self.binder.get(id).is_some_and(|node| node.kind.is_root()) {
            self.dragging = None;
            return;
        }
        self.dragging = Some(id);
        self.drop_target = None;
    }

    pub fn set_drop_target(&mut self, id: Id) {
        if self.dragging.is_some_and(|dragged| dragged != id) {
            self.drop_target = Some(id);
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging = None;
        self.drop_target = None;
    }

    pub fn drop_on(&mut self, target: Id) {
        let Some(dragged) = self.dragging.take() else {
            self.drop_target = None;
            return;
        };
        self.drop_target = None;
        self.flush_editor();
        match self.binder.drop_on(dragged, target) {
            Ok(()) => {
                if self.persist_binder() {
                    self.status = "Binder updated locally.".into();
                    if self.selected == Some(dragged) {
                        self.select_node(dragged);
                    }
                }
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    pub fn trash_selected(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.flush_editor();
        match self.binder.trash(id) {
            Ok(()) => {
                if self.persist_binder() {
                    self.status = "Moved to trash. Restore from the Trash folder or the inspector.".into();
                    if let Some(trash) = self.binder.trash_root() {
                        let trash_id = trash.id;
                        self.select_node(trash_id);
                    }
                }
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    pub fn restore_selected(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.flush_editor();
        match self.binder.restore(id) {
            Ok(()) => {
                if self.persist_binder() {
                    self.status = "Restored from trash.".into();
                    self.select_node(id);
                }
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    pub fn empty_trash(&mut self) {
        if !self.confirm_empty_trash {
            self.confirm_empty_trash = true;
            self.status = "Click Empty trash again to permanently delete trashed items on disk.".into();
            return;
        }
        self.confirm_empty_trash = false;
        self.flush_editor();
        let selected_in_trash = self
            .selected
            .is_some_and(|id| self.binder.is_in_trash(id));
        let (nodes, documents) = self.binder.purge_trash();
        if nodes.is_empty() {
            self.status = "Trash is already empty.".into();
            return;
        }
        if let Some(store) = &self.store {
            let store = store.borrow();
            for id in &documents {
                if let Err(err) = store.delete_document(*id) {
                    self.status = format!("Could not delete document: {err}");
                    return;
                }
            }
            for id in &nodes {
                if let Err(err) = store.delete_node(*id) {
                    self.status = format!("Could not delete binder item: {err}");
                    return;
                }
            }
        }
        if !self.persist_binder() {
            return;
        }
        if selected_in_trash {
            if let Some(trash) = self.binder.trash_root() {
                let trash_id = trash.id;
                self.select_node(trash_id);
            }
        }
        self.status = format!("Permanently deleted {} item(s) from local trash.", nodes.len());
    }

    fn infer_parent(&self, kind: NodeKind) -> Option<Id> {
        let selected = self.selected.and_then(|id| self.binder.get(id));
        match kind {
            NodeKind::Note => self.binder.research_root().map(|node| node.id),
            NodeKind::Scene => selected.and_then(|node| match node.kind {
                NodeKind::Chapter => Some(node.id),
                NodeKind::Scene => node.parent_id,
                _ => self
                    .binder
                    .nodes
                    .iter()
                    .rev()
                    .find(|node| node.kind == NodeKind::Chapter && !self.binder.is_in_trash(node.id))
                    .map(|node| node.id),
            }),
            NodeKind::Chapter => selected.and_then(|node| match node.kind {
                NodeKind::Book | NodeKind::Part => Some(node.id),
                NodeKind::Chapter | NodeKind::Scene => self
                    .binder
                    .nodes
                    .iter()
                    .find(|candidate| candidate.kind == NodeKind::Book)
                    .map(|book| book.id),
                _ => self.binder.draft_root().map(|node| node.id),
            }),
            NodeKind::Folder => selected.map(|node| node.id),
            _ => self.binder.draft_root().map(|node| node.id),
        }
    }

    pub fn update_selected_title(&mut self, title: String) {
        if let Some(id) = self.selected {
            if let Some(node) = self.binder.get_mut(id) {
                node.title = title;
                node.updated_at = writing_studio_core::Timestamp::now();
            }
            self.binder_dirty = true;
            self.note_edit();
        }
    }

    pub fn update_selected_synopsis(&mut self, synopsis: String) {
        if let Some(id) = self.selected {
            if let Some(node) = self.binder.get_mut(id) {
                node.synopsis = synopsis;
                node.updated_at = writing_studio_core::Timestamp::now();
            }
            self.binder_dirty = true;
            self.note_edit();
        }
    }

    pub fn update_selected_status(&mut self, status: String) {
        if let Some(id) = self.selected {
            if let Some(node) = self.binder.get_mut(id) {
                node.status = status;
                node.updated_at = writing_studio_core::Timestamp::now();
            }
            self.binder_dirty = true;
            self.note_edit();
        }
    }
}

fn sanitize(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed.to_string()
    }
}
