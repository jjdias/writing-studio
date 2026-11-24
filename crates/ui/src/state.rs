use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use writing_studio_document::{
    accept_change, compare_documents, BlockKind, reject_change, tracked_changes, Document, DocumentDiff,
    NavItem, NoteKind, ReviewItem, ReviewKind, ReviewTarget, TrackedChange,
};
use writing_studio_diagnostics::{
    findings, mentions, scene_facts, typical_words, BookContext, Finding, SceneBeats, SceneFacts,
};
use writing_studio_core::{autosave_due, autosave_wait, local_day, Id, Timestamp, PRODUCT_NAME};
use writing_studio_editor::EditorSession;
use writing_studio_manuscript::{book_outline_words, outline_rows};
use writing_studio_plotting::PlotGrid;
use writing_studio_project::{
    binder_color_hex, format_elapsed, keyword_covers, normalize_binder_color, MindMap, same_keyword, KeywordQuery, KeywordRow, KeywordSort, pace, tidy_keyword, BinderNode, BinderTree, NodeKind, Pace, PanelLayout,
    DropPlace, KeepPlace, MarkdownOptions, PageStyle, ProjectMeta, ProseFont, SavedFilter, StoredSession, Theme,
};
use writing_studio_search::ManuscriptIndex;
use writing_studio_spellcheck::{
    available_dictionaries, name_words, normalize_word, replace_in_document, DictionaryInfo,
    Speller, UnknownWord,
};
use writing_studio_synchronization::git::{self, Revision, SyncReport, SyncStatus};
use writing_studio_storage::{scene_document, BackupInfo, ProjectStore, SetAside, SnapshotInfo};

mod compile;
mod restructure;

pub use compile::{CheckFinding, CompileCheck};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceMode {
    Writing,
    MindMap,
    Outliner,
    Plot,
    History,
}

impl WorkspaceMode {
    /// The activity bar's order, which Ctrl+1 to Ctrl+5 follow.
    pub const ALL: [WorkspaceMode; 5] = [
        WorkspaceMode::Writing,
        WorkspaceMode::MindMap,
        WorkspaceMode::Outliner,
        WorkspaceMode::Plot,
        WorkspaceMode::History,
    ];

    /// The views listed in the binder under "Views". The page opens from a
    /// scene or a note.
    pub const LIBRARY_VIEWS: [WorkspaceMode; 4] =
        [WorkspaceMode::MindMap, WorkspaceMode::Outliner, WorkspaceMode::Plot, WorkspaceMode::History];

    /// The scene list is beside the page in Writing, and in History, which
    /// is about the scene chosen there. The mind map and the outliner show the
    /// scenes themselves; the library is in every mode.
    pub fn shows_binder(self) -> bool {
        matches!(self, Self::Writing | Self::History)
    }

    /// Context shows the item being written, or the node chosen on the
    /// mind map or in the outline.
    pub fn shows_inspector(self) -> bool {
        matches!(self, Self::Writing | Self::MindMap | Self::Outliner)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Writing => "Writing",
            Self::MindMap => "Mind map",
            Self::Outliner => "Outline",
            Self::Plot => "Plot",
            Self::History => "History",
        }
    }
}

/// One row of the binder: a book, part, chapter, folder, or scene, with
/// what the row shows of it.
#[derive(Clone, Debug, PartialEq)]
pub struct BinderRow {
    pub id: Id,
    pub depth: u32,
    pub kind: NodeKind,
    pub title: String,
    pub has_children: bool,
    pub collapsed: bool,
    /// A scene's status; empty for the rest.
    pub status: String,
    /// Its keywords, each with its color's hex, or `""` for none.
    pub keywords: Vec<(String, String)>,
    /// Compile leaves it out, by itself or with what it is in.
    pub left_out: bool,
    /// For what holds scenes: how many it holds, and how many are final.
    pub scenes: usize,
    pub finished: usize,
    /// Under Notes: whether it is compiled with the book, as footnotes.
    /// `None` elsewhere.
    pub compiles: Option<bool>,
}

/// A binder search: does the item, or something in it, match the words, by
/// its title, label, status, keywords, or text (`hits`, from the index)?
pub(crate) fn binder_matches(tree: &BinderTree, id: Id, query: &str, hits: &HashSet<Id>) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let Some(node) = tree.get(id) else {
        return false;
    };
    if writing_studio_search::matches_query(&node.title, query)
        || writing_studio_search::matches_query(&node.label, query)
        || writing_studio_search::matches_query(&node.status, query)
        || node.keywords.iter().any(|word| writing_studio_search::matches_query(word, query))
    {
        return true;
    }
    if node.document_id.is_some_and(|document_id| hits.contains(&document_id)) {
        return true;
    }
    tree.children(Some(id)).iter().any(|child| binder_matches(tree, child.id, query, hits))
}

/// A menu or small panel opened from the top bar or the status bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameMenu {
    Project,
    Settings,
    /// Shortcuts and the version, from the binder's Help menu.
    Help,
    Goals,
    /// The paragraph style menu on the format toolbar.
    Heading,
    /// The format toolbar's less common marks.
    More,
    /// The binder's Book, Part, and Folder.
    Add,
    /// The form for a new saved filter.
    Filter,
}

/// What the status line's Undo button can take back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UndoOffer {
    /// The last merge or split, through Ctrl+Z.
    Structure,
    /// An item moved (to the trash, or by dragging), put back where it
    /// was.
    PutBack { id: Id, parent: Id, after: Option<Id>, reopen: bool },
    /// An item added on the mind map, moved to the trash again.
    Added { id: Id },
    /// A keyword renamed or merged, from `StudioState::keyword_undo`.
    Keywords,
}

/// The binder's keywords before a rename, to put back with Undo.
#[derive(Clone, Debug)]
pub struct KeywordSnapshot {
    items: Vec<(Id, Vec<String>)>,
    keywords: Vec<writing_studio_project::Keyword>,
    filters: Vec<SavedFilter>,
    chosen: Option<String>,
}

/// A book, part, chapter, folder, or scene the Go to box can open.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickMatch {
    pub id: Id,
    pub kind: NodeKind,
    pub title: String,
    /// What it is in: "Book One › Chapter 2".
    pub place: String,
}

/// Where a right-click opened the menu of a binder item, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemMenu {
    pub id: Id,
    pub x: f64,
    pub y: f64,
}

/// A tab of the review panel under or beside the manuscript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewTab {
    Notes,
    Changes,
    Spelling,
    /// The scene's headings, links, and notes, to jump to.
    Navigate,
}

impl ReviewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Notes => "Notes",
            Self::Changes => "Changes",
            Self::Spelling => "Spelling",
            Self::Navigate => "Navigate",
        }
    }

    /// The tab to show: the one chosen while it has something in it, or
    /// else the first that does.
    pub fn shown(chosen: Option<ReviewTab>, available: &[ReviewTab]) -> Option<ReviewTab> {
        chosen
            .filter(|tab| available.contains(tab))
            .or_else(|| available.first().copied())
    }
}

/// A side panel that has folded away in a narrow window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldedPanel {
    Binder,
    Inspector,
}

/// Rich text pasted on the Markdown page, as Markdown: read as the
/// rich-text page read what was pasted into it, and written as the Markdown
/// export writes it. Only what was copied is read: the fragment a browser
/// or word processor marks, without its styles, scripts, and title. One
/// paragraph comes back as a line, to go into the paragraph pasted in.
pub fn pasted_markdown(html: &str) -> String {
    let fragment = match (html.find("<!--StartFragment-->"), html.find("<!--EndFragment-->")) {
        (Some(start), Some(end)) if start < end => &html[start + "<!--StartFragment-->".len()..end],
        _ => html,
    };
    let mut cleaned = fragment.to_string();
    for tag in ["head", "style", "script", "title", "template"] {
        cleaned = without_element(&cleaned, tag);
    }
    let mut document = Document::new(Id::new());
    document.replace_html(&cleaned);
    let empty = document.blocks.iter().all(|block| {
        matches!(&block.kind, BlockKind::Paragraph { inlines } if inlines.iter().all(|inline| inline.text.trim().is_empty()))
    });
    if empty {
        return String::new();
    }
    writing_studio_format::blocks_to_markdown(&document.blocks).trim_end_matches('\n').to_string()
}

/// `html` without every `<tag …>…</tag>` and what is in it, whatever the case.
fn without_element(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::new();
    let mut at = 0;
    while let Some(found) = lower[at..].find(&open) {
        let start = at + found;
        // `<header>` is not `<head>`.
        let next = lower[start + open.len()..].chars().next();
        if next.is_some_and(|ch| ch.is_ascii_alphanumeric()) {
            out.push_str(&html[at..start + open.len()]);
            at = start + open.len();
            continue;
        }
        out.push_str(&html[at..start]);
        at = match lower[start..].find(&close) {
            Some(end) => start + end + close.len(),
            None => html.len(),
        };
    }
    out.push_str(&html[at..]);
    out
}

/// An earlier version of the selected scene that is not a snapshot: the
/// scene at a sync, by its commit, or a version changed outside the program
/// and kept beside the scene's file, by that file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Earlier {
    Sync(String),
    Kept(String),
}

/// One side of a snapshot comparison is always a snapshot. The other is the
/// text on screen, or a second snapshot of the same document.
#[derive(Clone, Debug)]
pub struct SnapshotComparison {
    pub older: String,
    pub newer: String,
    pub diff: DocumentDiff,
}

/// The open document as it was when tracking was turned on. Tracked changes
/// are measured from this copy.
#[derive(Clone, Debug)]
pub struct Tracking {
    pub baseline: Document,
    pub started: Timestamp,
}

impl Tracking {
    pub fn started_label(&self) -> String {
        self.started.inner().format("%Y-%m-%d %H:%M UTC").to_string()
    }
}

/// One of the three things a scene is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beat {
    Goal,
    Conflict,
    Outcome,
}

/// What diagnostics says about one scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneReport {
    pub beats: SceneBeats,
    pub facts: SceneFacts,
    pub findings: Vec<Finding>,
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
    pub editor_epoch: u64,
    pub search: String,
    pub manuscript_index: Rc<RefCell<ManuscriptIndex>>,
    /// For each item with text, the note titles its text links to with
    /// `[[…]]`, in lowercase: filled when the project opens, kept up as the
    /// open text changes, so backlinks never read every file.
    pub link_index: HashMap<Id, Vec<String>>,
    /// The first line of each item's text, for a link's tip.
    pub openings: HashMap<Id, String>,
    /// The mind map's text filter: what does not match is dimmed.
    pub map_query: String,
    /// Mind map branches folded, until the program closes.
    pub folded_map: Vec<Id>,
    /// The mind map's zoom: `None` fits the window, a number scales it.
    pub map_zoom: Option<f64>,
    /// The mind map's ideas, lines, and hand-placed nodes (`mindmap.json`).
    pub mindmap: MindMap,
    /// The idea chosen on the mind map; a binder node chosen
    /// there is `selected`, as anywhere.
    pub map_focus: Option<Id>,
    /// The node a line is being drawn from, until the next node clicked.
    pub map_join: Option<Id>,
    /// Where on screen a drag on the map began, to tell it from a click.
    pub map_drag_start: Option<(f64, f64)>,
    /// Whether the mind map shows the notes instead of the book.
    pub map_notes: bool,
    pub plot: PlotGrid,
    pub plot_dirty: bool,
    pub confirm_plotline: Option<Id>,
    /// The plotline open beside the list in Plot.
    pub selected_plotline: Option<Id>,
    /// The book, part, chapter, folder, or trash chosen in the library,
    /// whose scenes the scene list shows.
    pub library_selection: Option<Id>,
    /// The saved filter chosen in the library, whose matches the scene list
    /// shows in place of a container's scenes.
    pub filter_selection: Option<Id>,
    /// The saved filter the filter form is changing, or `None` for a new one.
    pub editing_filter: Option<Id>,
    /// The keyword the scene list, the mind map, and the outliner are
    /// narrowed to, chosen in the library or their keyword menu.
    pub keyword_filter: Option<String>,
    /// Entries the open scene's text names that it is not linked to, found
    /// when it is opened or saved (`refresh_named_here`).
    pub named_here: Vec<Id>,
    /// Suggestions hidden, by scene and entry, until the program closes.
    hidden_named: Vec<(Id, Id)>,
    /// How the library's keyword list is ordered.
    pub keyword_sort: KeywordSort,
    /// Keywords folded in the library's list, whose children are hidden.
    pub folded_keywords: Vec<String>,
    /// The keyword being renamed in the library's list.
    pub renaming_keyword: Option<String>,
    /// A rename that would merge two keywords, waiting for a second Enter.
    confirm_keyword_merge: Option<(String, String)>,
    /// What the last keyword rename changed, for the status line's Undo.
    keyword_undo: Option<KeywordSnapshot>,
    pub mode: WorkspaceMode,
    pub focus_mode: bool,
    pub open_menu: Option<FrameMenu>,
    /// The menu a right-click opened on a library row or a scene card.
    pub item_menu: Option<ItemMenu>,
    /// The library row whose name is being edited.
    pub renaming: Option<Id>,
    /// The window that lists every shortcut.
    pub show_shortcuts: bool,
    /// The Go to box (Ctrl+P) is open.
    pub quick_open: bool,
    /// The Compile dialog is open.
    pub compile_open: bool,
    /// What the check before compiling found, shown in the Compile dialog
    /// until the book is compiled or the dialog is closed.
    pub compile_check: Option<compile::CompileCheck>,
    /// The book as one web page, shown in the Compile dialog in place of
    /// the profile while it is open.
    pub compile_preview: Option<String>,
    /// Scenes picked together with Ctrl or Shift in the scene list, for one
    /// action on all of them.
    pub picked: Vec<Id>,
    /// What the Undo button in the status line undoes, with the message it
    /// belongs to: it shows only while that message does.
    undo_offer: Option<(UndoOffer, String)>,
    /// A folded panel shown over the page until it is closed.
    pub overlay: Option<FoldedPanel>,
    /// The review panel's chosen tab, whether it is folded to its tab row,
    /// and whether it sits beside the manuscript instead of under it.
    pub review_tab: Option<ReviewTab>,
    pub review_folded: bool,
    pub review_at_side: bool,
    pub status: String,
    pub new_title: String,
    pub confirm_backup: Option<PathBuf>,
    pub compare_snapshot: Option<Id>,
    pub compare_against: Option<Id>,
    pub compare_changes_only: bool,
    /// An earlier version of the selected scene that is being compared.
    pub compare_revision: Option<Earlier>,
    /// A kept file waiting for a second click to be deleted.
    pub confirm_kept: Option<String>,
    pub show_resolved_notes: bool,
    pub tracking: Option<Tracking>,
    /// What undo puts back besides the text after a merge or a split.
    structure_step: Option<restructure::StructureStep>,
    /// The dictionary in use. `None` while spellcheck is off or before a project is open.
    pub speller: Option<Rc<Speller>>,
    /// The unknown word whose suggestions are on screen.
    pub spell_focus: Option<String>,
    pub show_project_words: bool,
    /// The address typed in to connect the project for sync.
    pub sync_address: String,
    /// A sync is running; the manuscript cannot be edited until it ends.
    pub syncing: bool,
    /// What the last sync could not combine, one line each.
    pub sync_conflicts: Vec<String>,
    /// Goal, conflict, and outcome by binder item.
    pub beats: HashMap<Id, SceneBeats>,
    pub beats_dirty: bool,
    /// The outliner shows the diagnostics table instead of the outline.
    pub outliner_diagnostics: bool,
    /// Scene text by document, read when the diagnostics table opens.
    pub diagnostic_texts: Rc<HashMap<Id, String>>,
    pub binder_dirty: bool,
    pub last_edit: Option<Instant>,
    pub session_started: Option<Instant>,
    pub session_baseline: u32,
    pub session_shown_secs: u64,
    pub settings_dirty: bool,
    pub dragging: Option<Id>,
    pub drop_target: Option<Id>,
    /// Where on the binder row under the pointer the dragged item would go.
    pub drop_place: DropPlace,
    pub ignore_next_click: bool,
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
            editor_epoch: 0,
            search: String::new(),
            manuscript_index: Rc::new(RefCell::new(ManuscriptIndex::new())),
            link_index: HashMap::new(),
            openings: HashMap::new(),
            map_query: String::new(),
            folded_map: Vec::new(),
            map_zoom: None,
            mindmap: MindMap::default(),
            map_focus: None,
            map_join: None,
            map_drag_start: None,
            map_notes: false,
            plot: PlotGrid::new(),
            plot_dirty: false,
            confirm_plotline: None,
            selected_plotline: None,
            library_selection: None,
            filter_selection: None,
            editing_filter: None,
            keyword_filter: None,
            named_here: Vec::new(),
            hidden_named: Vec::new(),
            keyword_sort: KeywordSort::Name,
            folded_keywords: Vec::new(),
            renaming_keyword: None,
            confirm_keyword_merge: None,
            keyword_undo: None,
            mode: WorkspaceMode::Writing,
            focus_mode: false,
            open_menu: None,
            item_menu: None,
            renaming: None,
            show_shortcuts: false,
            quick_open: false,
            compile_open: false,
            compile_check: None,
            compile_preview: None,
            picked: Vec::new(),
            undo_offer: None,
            overlay: None,
            review_tab: None,
            review_folded: false,
            review_at_side: false,
            status: "Offline · Local-first".into(),
            new_title: "Untitled Novel".into(),
            confirm_backup: None,
            compare_snapshot: None,
            compare_against: None,
            compare_changes_only: false,
            compare_revision: None,
            confirm_kept: None,
            show_resolved_notes: false,
            tracking: None,
            structure_step: None,
            speller: None,
            spell_focus: None,
            show_project_words: false,
            sync_address: String::new(),
            syncing: false,
            sync_conflicts: Vec::new(),
            beats: HashMap::new(),
            beats_dirty: false,
            outliner_diagnostics: false,
            diagnostic_texts: Rc::new(HashMap::new()),
            binder_dirty: false,
            last_edit: None,
            session_started: None,
            session_baseline: 0,
            session_shown_secs: 0,
            settings_dirty: false,
            dragging: None,
            drop_target: None,
            drop_place: DropPlace::After,
            ignore_next_click: false,
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
        self.manuscript_words(true)
    }

    fn manuscript_words(&self, include_open_edits: bool) -> u32 {
        let Some(store) = &self.store else {
            return 0;
        };
        let store = store.borrow();
        let open = if include_open_edits {
            self.session
                .as_ref()
                .map(|session| (session.document_id(), session.word_count()))
        } else {
            None
        };
        self.binder.total_word_count(|id| match open {
            Some((open_id, words)) if open_id == id => words,
            _ => store.document_word_count(id),
        })
    }

    pub fn goal(&self) -> u32 {
        let Some(book) = self.compile_book_id() else {
            return self
                .meta
                .as_ref()
                .map(|meta| meta.settings.project_word_goal)
                .unwrap_or(80_000);
        };
        let first = self
            .binder
            .manuscript_books()
            .first()
            .is_some_and(|node| node.id == book);
        self.meta
            .as_ref()
            .map(|meta| meta.settings.goal_for_book(book, first))
            .unwrap_or(80_000)
    }

    /// The book Markdown and DOCX export, and the book whose words and target are shown.
    pub fn compile_book_id(&self) -> Option<Id> {
        let books = self.binder.manuscript_books();
        if let Some(stored) = self
            .meta
            .as_ref()
            .and_then(|meta| meta.settings.compile_book_id)
        {
            if books.iter().any(|book| book.id == stored) {
                return Some(stored);
            }
        }
        books.first().map(|book| book.id)
    }

    pub fn compile_book_title(&self) -> String {
        self.compile_book_id()
            .and_then(|id| self.binder.get(id))
            .map(|node| node.title.clone())
            .unwrap_or_else(|| "Project".into())
    }

    pub fn compile_book_word_count(&self) -> u32 {
        let Some(book) = self.compile_book_id() else {
            return 0;
        };
        self.outline_words(book, true)
    }

    pub fn current_book_words_today(&self) -> u32 {
        let Some(book) = self.compile_book_id() else {
            return 0;
        };
        let words = self.outline_words(book, true);
        self.meta
            .as_ref()
            .map(|meta| meta.settings.writing.words_today_for_book(book, words))
            .unwrap_or(0)
    }

    pub fn current_book_words_this_session(&self) -> u32 {
        let Some(book) = self.compile_book_id() else {
            return 0;
        };
        let words = self.outline_words(book, true);
        self.meta
            .as_ref()
            .map(|meta| meta.settings.writing.session_words_for_book(book, words))
            .unwrap_or(0)
    }

    fn outline_words(&self, book: Id, include_open_edits: bool) -> u32 {
        book_outline_words(&self.binder, book, |id| {
            self.document_words(id, include_open_edits)
        })
    }

    fn book_outline_counts(&self, include_open_edits: bool) -> Vec<(Id, u32)> {
        self.binder
            .manuscript_books()
            .iter()
            .map(|book| (book.id, self.outline_words(book.id, include_open_edits)))
            .collect()
    }

    pub fn manuscript_book_choices(&self) -> Vec<(Id, String)> {
        self.binder
            .manuscript_books()
            .into_iter()
            .map(|node| (node.id, node.title.clone()))
            .collect()
    }

    /// The book the binder, mind map, outliner, and search are limited to.
    /// None means the whole series is on screen. Compile still uses [`Self::compile_book_id`].
    pub fn series_scope(&self) -> Option<Id> {
        let show_all = self
            .meta
            .as_ref()
            .map(|meta| meta.settings.show_whole_series)
            .unwrap_or(true);
        if show_all {
            None
        } else {
            self.compile_book_id()
        }
    }

    /// Whole series shows every book and leaves the compile book unchanged.
    /// Choosing a book focuses that book in the filter, the compile menu, and the status bar.
    pub fn set_series_scope(&mut self, book: Option<Id>) {
        match book {
            None => self.set_show_whole_series(true),
            Some(id) => self.set_compile_book(id),
        }
    }

    pub fn set_compile_book(&mut self, id: Id) {
        self.flush_editor();
        self.remember_compile_book(id);
        self.set_show_whole_series(false);
    }

    /// A scene in the current book, or the first scene in the manuscript.
    fn opening_scene(&self) -> Option<Id> {
        let in_current_book = self.compile_book_id().and_then(|book| {
            self.binder.nodes.iter().find(|node| {
                node.kind == NodeKind::Scene
                    && !self.binder.is_in_trash(node.id)
                    && self.binder.book_of(node.id) == Some(book)
            })
        });
        in_current_book
            .or_else(|| {
                self.binder.nodes.iter().find(|node| {
                    node.kind == NodeKind::Scene && !self.binder.is_in_trash(node.id)
                })
            })
            .map(|node| node.id)
    }

    /// Point compile and the status bar at the book that contains `id`, without changing the series view.
    fn adopt_book_of(&mut self, id: Id) {
        if let Some(book) = self.binder.book_of(id) {
            self.remember_compile_book(book);
        }
    }

    fn remember_compile_book(&mut self, id: Id) {
        if !self
            .binder
            .manuscript_books()
            .iter()
            .any(|book| book.id == id)
        {
            return;
        }
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.compile_book_id == Some(id) {
                return false;
            }
            meta.settings.compile_book_id = Some(id);
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
        let needs_baseline = self.meta.as_ref().is_none_or(|meta| {
            meta.settings
                .writing
                .books
                .iter()
                .find(|book| book.book_id == id)
                .map(|book| book.day != local_day() || book.session_baseline.is_none())
                .unwrap_or(true)
        });
        if needs_baseline {
            self.track_compile_book(id);
        }
    }

    fn track_compile_book(&mut self, id: Id) {
        let words = self.outline_words(id, false);
        let today = local_day();
        let Some(meta) = &mut self.meta else {
            return;
        };
        let before = meta.settings.writing.clone();
        meta.settings.writing.track_book(id, &today, words);
        if meta.settings.writing != before {
            meta.touch();
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    fn set_show_whole_series(&mut self, show: bool) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.show_whole_series == show {
                return false;
            }
            meta.settings.show_whole_series = show;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    fn document_words(&self, id: Id, include_open_edits: bool) -> u32 {
        if include_open_edits {
            if let Some(session) = &self.session {
                if session.document_id() == id {
                    return session.word_count();
                }
            }
        }
        self.store
            .as_ref()
            .map(|store| store.borrow().document_word_count(id))
            .unwrap_or(0)
    }

    pub fn daily_goal(&self) -> u32 {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.daily_word_goal)
            .unwrap_or(500)
    }

    pub fn words_today_at(&self, current: u32) -> u32 {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.writing.words_today(current))
            .unwrap_or(0)
    }

    pub fn words_this_session_at(&self, current: u32) -> u32 {
        if self.session_started.is_none() {
            return 0;
        }
        current.saturating_sub(self.session_baseline)
    }

    pub fn session_elapsed_secs(&self) -> u64 {
        self.session_started
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn stored_sessions(&self) -> Vec<StoredSession> {
        let mut sessions = self
            .meta
            .as_ref()
            .map(|meta| meta.settings.writing.sessions.clone())
            .unwrap_or_default();
        sessions.reverse();
        sessions
    }

    pub fn set_daily_goal(&mut self, goal: u32) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.daily_word_goal == goal {
                return false;
            }
            meta.settings.daily_word_goal = goal;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn panel_layout(&self) -> PanelLayout {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.layout.clamp())
            .unwrap_or_default()
    }

    pub fn store_layout(&mut self, layout: PanelLayout) {
        let layout = layout.clamp();
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.layout == layout {
                return false;
            }
            meta.settings.layout = layout;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn theme(&self) -> Theme {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.theme)
            .unwrap_or_default()
    }

    pub fn prose_font(&self) -> ProseFont {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.prose_font)
            .unwrap_or_default()
    }

    pub fn page_style(&self) -> PageStyle {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.page)
            .unwrap_or_default()
    }

    /// Set how the page looks while writing, kept within its limits. It is
    /// this computer's choice, saved with the font and theme.
    pub fn set_page_style(&mut self, style: PageStyle) {
        let style = style.clamp();
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.page == style {
                return false;
            }
            meta.settings.page = style;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn keep_place(&self) -> KeepPlace {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.keep_place)
            .unwrap_or_default()
    }

    pub fn markdown_options(&self) -> MarkdownOptions {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.markdown)
            .unwrap_or_default()
    }

    /// Source mode, smart typography, and pairing, saved in the project.
    pub fn set_markdown_options(&mut self, options: MarkdownOptions) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.markdown == options {
                return false;
            }
            meta.settings.markdown = options;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    /// Show every Markdown mark, or only those where the caret is.
    pub fn toggle_source_mode(&mut self) {
        let mut options = self.markdown_options();
        options.source_mode = !options.source_mode;
        self.set_markdown_options(options);
        self.status = if options.source_mode {
            "Source mode: every Markdown mark shows.".into()
        } else {
            "Live preview: marks show where the caret is.".into()
        };
    }

    /// Fixed scrolling, the line marker, and fading: this computer's choice.
    pub fn set_keep_place(&mut self, keep: KeepPlace) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.keep_place == keep {
                return false;
            }
            meta.settings.keep_place = keep;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn set_prose_font(&mut self, font: ProseFont) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.prose_font == font {
                return false;
            }
            meta.settings.prose_font = font;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.theme == theme {
                return false;
            }
            meta.settings.theme = theme;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn set_project_goal(&mut self, goal: u32) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.project_word_goal == goal {
                return false;
            }
            meta.settings.project_word_goal = goal;
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    /// The current book's deadline, as `YYYY-MM-DD`.
    pub fn compile_book_deadline(&self) -> Option<String> {
        let book = self.compile_book_id()?;
        self.meta
            .as_ref()?
            .settings
            .deadline_for_book(book)
            .map(str::to_string)
    }

    /// Set or clear the current book's deadline. It goes with the book's
    /// target in `project.json`, so every machine sees it.
    pub fn set_compile_book_deadline(&mut self, deadline: Option<String>) {
        let Some(book) = self.compile_book_id() else {
            self.status = "Add a book to give it a deadline.".into();
            return;
        };
        let deadline = deadline.filter(|day| writing_studio_core::days_between(day, day).is_some());
        let first = self
            .binder
            .manuscript_books()
            .first()
            .is_some_and(|node| node.id == book);
        let changed = self.meta.as_mut().is_some_and(|meta| {
            if meta.settings.deadline_for_book(book) == deadline.as_deref() {
                return false;
            }
            meta.settings.set_deadline_for_book(book, deadline, first);
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    /// Where the current book stands against its deadline, today.
    pub fn compile_book_pace(&self) -> Option<Pace> {
        let deadline = self.compile_book_deadline()?;
        pace(
            self.goal(),
            self.compile_book_word_count(),
            &writing_studio_core::local_day(),
            &deadline,
        )
    }

    pub fn set_compile_book_goal(&mut self, goal: u32) {
        let Some(book) = self.compile_book_id() else {
            self.set_project_goal(goal);
            return;
        };
        let first = self
            .binder
            .manuscript_books()
            .first()
            .is_some_and(|node| node.id == book);
        let changed = self.meta.as_mut().is_some_and(|meta| {
            let same_goal = meta.settings.goal_for_book(book, first) == goal;
            let same_project = !first || meta.settings.project_word_goal == goal;
            if same_goal && same_project {
                return false;
            }
            meta.settings.set_goal_for_book(book, goal, first);
            meta.touch();
            true
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
        }
    }

    pub fn selected_node(&self) -> Option<&BinderNode> {
        self.selected.and_then(|id| self.binder.get(id))
    }

    pub fn create_project_in(&mut self, folder: PathBuf) {
        let title = self.new_title.trim();
        let title = if title.is_empty() { "Untitled Novel" } else { title };
        let root = match ProjectStore::new_project_root(&folder, &format!("{}.writingstudio", sanitize(title))) {
            Ok(root) => root,
            Err(err) => {
                self.status = format!("Could not create the project here: {err}.");
                return;
            }
        };
        match ProjectStore::create(&root, title) {
            Ok(store) => self.attach_store(store, &format!("Created the project in {}.", root.display())),
            Err(err) => self.status = format!("Could not create project: {err}"),
        }
    }

    pub fn open_project_at(&mut self, folder: PathBuf) {
        match ProjectStore::open(&folder) {
            Ok(store) => self.attach_store(store, "Opened local project."),
            Err(err) => self.status = format!("Could not open project: {err}"),
        }
    }

    fn attach_store(&mut self, store: ProjectStore, status: &str) {
        match (
            store.load_meta(),
            store.load_binder(),
            store.load_plot(),
        ) {
            (Ok(meta), Ok(binder), Ok(plot)) => {
                self.meta = Some(meta);
                self.binder = binder;
                self.plot = plot;
                self.plot_dirty = false;
                self.confirm_plotline = None;
                self.mindmap = match store.load_mindmap() {
                    Ok(map) => map,
                    Err(err) => {
                        self.status = format!("Could not read the mind map: {err}");
                        MindMap::default()
                    }
                };
                self.map_focus = None;
                self.map_join = None;
                self.store = Some(Rc::new(RefCell::new(store)));
                self.status = status.into();
                if let Some(id) = self.opening_scene() {
                    self.select_node(id);
                }
                self.reindex_manuscript();
                self.begin_writing_session();
                self.load_speller();
                self.load_beats();
                self.note_open_problems();
            }
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
                self.status = format!("Project load failed: {err}");
            }
        }
    }

    pub fn select_node(&mut self, id: Id) {
        self.flush_editor();
        // A new editor session has no undo for the last merge or split.
        self.structure_step = None;
        if self.selected != Some(id) {
            self.close_comparison();
        }
        self.selected = Some(id);
        self.adopt_book_of(id);
        let document_id = self.binder.get(id).and_then(|node| node.document_id);
        if let (Some(store), Some(doc_id)) = (&self.store, document_id) {
            let loaded = store.borrow().load_or_create_document(doc_id);
            match loaded {
                Ok(document) => {
                    let session = EditorSession::new(document);
                    self.sync_editor_from_session(&session);
                    self.session = Some(session);
                }
                Err(err) => self.status = format!("Could not load document: {err}"),
            }
        } else {
            self.session = None;
            self.editor_text.clear();
            self.editor_epoch += 1;
        }
        self.load_tracking();
        self.refresh_named_here();
    }

    /// Read the tracking baseline of the open document, if it has one.
    fn load_tracking(&mut self) {
        self.tracking = None;
        let (Some(store), Some(session)) = (&self.store, &self.session) else {
            return;
        };
        match store.borrow().load_tracking_baseline(session.document_id()) {
            Ok(found) => {
                self.tracking = found.map(|(baseline, started)| Tracking { baseline, started });
            }
            Err(err) => self.status = format!("Could not read tracked changes: {err}"),
        }
    }

    /// Read the dictionary the project asks for. A dictionary that is gone
    /// from disk falls back to the bundled one.
    fn load_speller(&mut self) {
        self.speller = None;
        self.spell_focus = None;
        let Some(meta) = &self.meta else {
            return;
        };
        if !meta.settings.spelling.enabled {
            return;
        }
        let wanted = meta.settings.spelling.language.clone();
        let choices = self.spelling_choices();
        let found = choices.iter().find(|item| item.id == wanted);
        let loaded = match found {
            Some(info) => Speller::load(info),
            None => Speller::bundled(),
        };
        match loaded {
            Ok(speller) => {
                if found.is_none() {
                    self.status =
                        format!("The {wanted} dictionary was not found. Checking spelling in English (US).");
                }
                self.speller = Some(Rc::new(speller));
            }
            Err(err) => self.status = format!("Spellcheck is off: {err}"),
        }
    }

    /// The bundled dictionary, then any found in the project's
    /// `dictionaries/` folder and on this computer.
    pub fn spelling_choices(&self) -> Vec<DictionaryInfo> {
        let root = self.store.as_ref().map(|store| store.borrow().root.clone());
        available_dictionaries(root.as_deref())
    }

    /// The dictionary name in use, or `None` when spellcheck is off.
    pub fn spelling_language(&self) -> Option<String> {
        self.speller.as_ref().map(|speller| speller.language().to_string())
    }

    /// Choose a dictionary by name, or `None` to turn spellcheck off.
    pub fn set_spelling(&mut self, language: Option<String>) {
        let changed = self.meta.as_mut().is_some_and(|meta| {
            let spelling = &mut meta.settings.spelling;
            let before = spelling.clone();
            spelling.enabled = language.is_some();
            if let Some(language) = language {
                spelling.language = language;
            }
            let changed = *spelling != before;
            if changed {
                meta.touch();
            }
            changed
        });
        if changed {
            self.settings_dirty = true;
            self.note_edit();
            self.load_speller();
        }
    }

    pub fn project_words(&self) -> Vec<String> {
        self.meta
            .as_ref()
            .map(|meta| meta.settings.spelling.words.clone())
            .unwrap_or_default()
    }

    /// Words spellcheck accepts besides the dictionary: the project's own
    /// list, and every word in a character, place, or culture name.
    /// Every note's title, under Notes and not in the trash: the names the
    /// spelling accepts, the diagnostics look for, and "Named here" offers.
    pub fn note_titles(&self) -> Vec<String> {
        self.binder
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Note && node.deleted_at.is_none() && self.binder.is_in_notes(node.id))
            .map(|node| node.title.clone())
            .filter(|title| !title.trim().is_empty())
            .collect()
    }

    fn accepted_words(&self) -> HashSet<String> {
        let titles = self.note_titles();
        let mut words = name_words(titles.iter().map(String::as_str));
        words.extend(self.project_words());
        words
    }

    /// Words in the open document that are not in the dictionary, in the
    /// order they first appear.
    pub fn unknown_words(&self) -> Vec<UnknownWord> {
        let (Some(speller), Some(session)) = (&self.speller, &self.session) else {
            return Vec::new();
        };
        speller.unknown_words(&session.plain_text(), &self.accepted_words())
    }

    pub fn spelling_suggestions(&self, word: &str) -> Vec<String> {
        self.speller
            .as_ref()
            .map(|speller| speller.suggest(word))
            .unwrap_or_default()
    }

    /// Accept a word everywhere in this project.
    pub fn add_project_word(&mut self, word: &str) {
        let Some(word) = normalize_word(word) else {
            return;
        };
        let added = self.meta.as_mut().is_some_and(|meta| {
            let words = &mut meta.settings.spelling.words;
            if words.contains(&word) {
                return false;
            }
            words.push(word.clone());
            words.sort();
            meta.touch();
            true
        });
        if added {
            self.settings_dirty = true;
            self.note_edit();
            self.spell_focus = None;
            self.status = format!("“{word}” is in this project's dictionary.");
        }
    }

    pub fn remove_project_word(&mut self, word: &str) {
        let removed = self.meta.as_mut().is_some_and(|meta| {
            let words = &mut meta.settings.spelling.words;
            let before = words.len();
            words.retain(|known| known != word);
            let removed = words.len() != before;
            if removed {
                meta.touch();
            }
            removed
        });
        if removed {
            self.settings_dirty = true;
            self.note_edit();
            self.status = format!("“{word}” was taken out of this project's dictionary.");
        }
    }

    /// Replace every whole-word `from` in the open document. One undo step.
    pub fn replace_misspelling(&mut self, from: &str, to: &str) {
        let mut count = 0;
        self.edit_text("", |doc| {
            count = replace_in_document(doc, from, to);
            count > 0
        });
        if count > 0 {
            self.spell_focus = None;
            self.status = format!("Replaced “{from}” with “{to}” {count} times in this document.");
        } else {
            self.status =
                format!("“{from}” was not replaced. A word that is partly formatted has to be fixed by hand.");
        }
    }

    fn load_beats(&mut self) {
        self.beats_dirty = false;
        self.diagnostic_texts = Rc::new(HashMap::new());
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().load_scene_beats() {
            Ok(beats) => self.beats = beats,
            Err(err) => {
                self.beats = HashMap::new();
                self.status = format!("Could not read scene goals: {err}");
            }
        }
    }

    fn persist_beats_now(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_scene_beats(&self.beats) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save scene goals: {err}");
                false
            }
        }
    }

    /// Whether diagnostics covers this binder item: a scene in the
    /// manuscript, or a chapter written without scenes.
    pub fn has_scene_diagnostics(&self, node_id: Id) -> bool {
        outline_rows(&self.binder)
            .iter()
            .any(|row| row.document_id.is_some() && row.scene_id.unwrap_or(row.chapter_id) == node_id)
    }

    pub fn scene_beats(&self, node_id: Id) -> SceneBeats {
        self.beats.get(&node_id).cloned().unwrap_or_default()
    }

    pub fn set_scene_beat(&mut self, node_id: Id, beat: Beat, text: String) {
        let entry = self.beats.entry(node_id).or_default();
        let slot = match beat {
            Beat::Goal => &mut entry.goal,
            Beat::Conflict => &mut entry.conflict,
            Beat::Outcome => &mut entry.outcome,
        };
        if *slot == text {
            return;
        }
        *slot = text;
        self.beats_dirty = true;
        self.note_edit();
    }

    /// Read every scene's text for the diagnostics table. The table asks for
    /// this when it opens, so it shows the text as last written.
    pub fn load_diagnostic_texts(&mut self) {
        self.flush_editor();
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().load_all_documents() {
            Ok(documents) => {
                self.diagnostic_texts = Rc::new(
                    documents
                        .into_iter()
                        .map(|document| (document.id, document.to_plain_text()))
                        .collect(),
                );
            }
            Err(err) => self.status = format!("Could not read scenes for diagnostics: {err}"),
        }
    }

    fn document_text(&self, document_id: Id) -> String {
        if let Some(session) = self.session.as_ref().filter(|s| s.document_id() == document_id) {
            return session.plain_text();
        }
        if let Some(text) = self.diagnostic_texts.get(&document_id) {
            return text.clone();
        }
        self.store
            .as_ref()
            .and_then(|store| store.borrow().load_document(document_id).ok())
            .map(|document| document.to_plain_text())
            .unwrap_or_default()
    }

    /// What a scene is compared with: the usual scene length in its book,
    /// and whether there are notes whose titles to look for.
    pub fn book_context(&self, book_id: Id) -> BookContext {
        let counts = outline_rows(&self.binder)
            .into_iter()
            .filter(|row| row.book_id == book_id)
            .filter_map(|row| row.document_id)
            .map(|document_id| self.document_words(document_id, true));
        BookContext {
            typical_words: typical_words(counts),
            has_cast: !self.note_titles().is_empty(),
        }
    }

    /// Diagnostics for one scene, measured against its own book.
    pub fn scene_report(&self, node_id: Id) -> Option<SceneReport> {
        let book = self.binder.book_of(node_id)?;
        self.scene_report_in(node_id, &self.book_context(book))
    }

    /// The same, when the caller already has the book's context.
    pub fn scene_report_in(&self, node_id: Id, context: &BookContext) -> Option<SceneReport> {
        let document_id = self.binder.get(node_id)?.document_id?;
        let names = self.note_titles();
        let beats = self.scene_beats(node_id);
        let facts = scene_facts(&self.document_text(document_id), &names);
        let findings = findings(&beats, &facts, context);
        Some(SceneReport {
            beats,
            facts,
            findings,
        })
    }

    /// The baseline, when it belongs to the document in the editor.
    fn tracking_for_session(&self) -> Option<(&Tracking, &EditorSession)> {
        let session = self.session.as_ref()?;
        let tracking = self.tracking.as_ref()?;
        (tracking.baseline.id == session.document_id()).then_some((tracking, session))
    }

    pub fn is_tracking(&self) -> bool {
        self.tracking_for_session().is_some()
    }

    /// Paragraphs rewritten, added, or removed since tracking began.
    pub fn tracked_changes(&self) -> Vec<TrackedChange> {
        self.tracking_for_session()
            .map(|(tracking, session)| tracked_changes(&tracking.baseline, session.document()))
            .unwrap_or_default()
    }

    /// Start tracking the open document from the text it has now.
    pub fn start_tracking(&mut self) {
        if self.is_tracking() {
            return;
        }
        self.flush_editor();
        let (Some(store), Some(session)) = (&self.store, &self.session) else {
            return;
        };
        let baseline = session.document().clone();
        let saved = store.borrow().save_tracking_baseline(&baseline);
        match saved {
            Ok(()) => {
                self.load_tracking();
                self.status = "Tracking changes to this document from now.".into();
            }
            Err(err) => self.status = format!("Could not start tracking: {err}"),
        }
    }

    /// Stop tracking. The text stays as it is; the baseline is discarded.
    pub fn stop_tracking(&mut self) {
        let (Some(store), Some(session)) = (&self.store, &self.session) else {
            return;
        };
        let stopped = store.borrow().stop_tracking(session.document_id());
        match stopped {
            Ok(()) => {
                self.tracking = None;
                self.status = "Stopped tracking. The text was kept as it is.".into();
            }
            Err(err) => self.status = format!("Could not stop tracking: {err}"),
        }
    }

    /// Keep one change. It leaves the list; the text does not move.
    pub fn accept_tracked_change(&mut self, index: usize) {
        let Some((tracking, session)) = self.tracking_for_session() else {
            return;
        };
        let Some(change) = tracked_changes(&tracking.baseline, session.document()).get(index).cloned()
        else {
            return;
        };
        let mut baseline = tracking.baseline.clone();
        if accept_change(&mut baseline, session.document(), &change) {
            self.replace_baseline(baseline, "Change accepted.");
        }
    }

    /// Keep every change: the baseline becomes the text as it is now.
    pub fn accept_all_tracked_changes(&mut self) {
        let Some((_, session)) = self.tracking_for_session() else {
            return;
        };
        let baseline = session.document().clone();
        self.replace_baseline(baseline, "All changes accepted.");
    }

    fn replace_baseline(&mut self, baseline: Document, status: &str) {
        let Some(store) = &self.store else {
            return;
        };
        let saved = store.borrow().save_tracking_baseline(&baseline);
        match saved {
            Ok(()) => {
                if let Some(tracking) = &mut self.tracking {
                    tracking.baseline = baseline;
                }
                self.status = status.into();
            }
            Err(err) => self.status = format!("Could not accept the change: {err}"),
        }
    }

    /// Undo one change in the text. It is one undo step.
    pub fn reject_tracked_change(&mut self, index: usize) {
        let Some((tracking, session)) = self.tracking_for_session() else {
            return;
        };
        let Some(change) = tracked_changes(&tracking.baseline, session.document()).get(index).cloned()
        else {
            return;
        };
        let baseline = tracking.baseline.clone();
        self.edit_text("Change rejected. Undo brings it back.", |doc| {
            reject_change(&baseline, doc, &change)
        });
    }

    /// Put the text back as it was when tracking began. One undo step.
    pub fn reject_all_tracked_changes(&mut self) {
        let Some((tracking, session)) = self.tracking_for_session() else {
            return;
        };
        if tracked_changes(&tracking.baseline, session.document()).is_empty() {
            return;
        }
        let baseline = tracking.baseline.clone();
        self.edit_text("All changes rejected. Undo brings them back.", |doc| {
            doc.blocks = baseline.blocks.clone();
            doc.updated_at = Timestamp::now();
            true
        });
    }

    /// Change the words of the open document from outside the editor, as one
    /// undo step, and redraw the editor.
    fn edit_text(&mut self, status: &str, change: impl FnOnce(&mut Document) -> bool) {
        let Some(mut session) = self.session.take() else {
            return;
        };
        let changed = session.edit(change);
        if changed {
            self.sync_editor_from_session(&session);
        }
        self.session = Some(session);
        if changed {
            self.note_edit();
            self.note_session_for_search();
            self.status = status.into();
        }
    }

    fn sync_editor_from_session(&mut self, session: &EditorSession) {
        self.editor_text = session.plain_text();
        self.editor_epoch = self.editor_epoch.saturating_add(1);
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

    /// Notes and highlights in the open document.
    /// The open scene's headings, links, and notes, from the document the
    /// editor holds, not from its file.
    pub fn navigation(&self) -> Vec<NavItem> {
        self.session
            .as_ref()
            .map(|session| session.document().navigation())
            .unwrap_or_default()
    }

    pub fn review_items(&self) -> Vec<ReviewItem> {
        self.session
            .as_ref()
            .map(|session| session.document().review_items())
            .unwrap_or_default()
    }

    pub fn resolve_note(&mut self, note: usize, resolved: bool) {
        let status = if resolved { "Note resolved." } else { "Note reopened." };
        self.edit_review(status, |doc| doc.set_note_resolved(note, resolved));
    }

    pub fn set_note_kind(&mut self, note: usize, kind: NoteKind) {
        self.edit_review("Note changed.", |doc| doc.set_note_kind(note, kind));
    }

    pub fn set_note_text(&mut self, note: usize, text: &str) {
        self.edit_review("Note changed.", |doc| doc.set_note_text(note, text));
    }

    pub fn remove_review(&mut self, target: ReviewTarget) {
        let status = match target {
            ReviewTarget::Note(_) => "Note removed. The text stays.",
            ReviewTarget::Highlight(_) => "Highlight removed. The text stays.",
        };
        self.edit_review(status, |doc| doc.remove_review(target));
    }

    /// Take every resolved note off the open document.
    pub fn remove_resolved_notes(&mut self) {
        self.edit_review("Resolved notes removed. The text stays.", |doc| {
            let mut changed = false;
            // Removing a note renumbers the ones after it, so look again each time.
            while let Some(target) = doc.review_items().into_iter().find_map(|item| {
                matches!(item.kind, ReviewKind::Note { resolved: true, .. }).then_some(item.target)
            }) {
                if !doc.remove_review(target) {
                    break;
                }
                changed = true;
            }
            changed
        });
    }

    /// A review edit changes marks, not text. It is one undo step, and the
    /// editor is redrawn from the document.
    fn edit_review(&mut self, status: &str, change: impl FnOnce(&mut Document) -> bool) {
        let Some(mut session) = self.session.take() else {
            return;
        };
        let changed = session.edit(change);
        if changed {
            self.sync_editor_from_session(&session);
        }
        self.session = Some(session);
        if changed {
            self.note_edit();
            self.status = status.into();
        }
    }

    /// The open scene as the Markdown page shows it: what the Markdown export
    /// writes, without its last line end, and nothing for an empty scene.
    pub fn editor_markdown(&self) -> String {
        let Some(session) = &self.session else {
            return String::new();
        };
        let blocks = &session.document().blocks;
        let empty = blocks
            .iter()
            .all(|block| matches!(&block.kind, BlockKind::Paragraph { inlines } if inlines.iter().all(|inline| inline.text.is_empty())));
        if empty {
            return String::new();
        }
        let mut text = writing_studio_format::blocks_to_markdown(blocks);
        if text.ends_with('\n') {
            text.pop();
        }
        text
    }

    /// Take what the Markdown page holds as the open scene's text.
    pub fn apply_editor_markdown(&mut self, text: &str) {
        let Some(session) = &mut self.session else {
            return;
        };
        if session.apply_blocks(writing_studio_format::markdown_to_blocks(text)) {
            self.editor_text = session.plain_text();
            self.note_edit();
            self.note_session_for_search();
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
                    let id = session.document_id();
                    let text = session.plain_text();
                    session.mark_clean();
                    match self.manuscript_index.borrow_mut().upsert(id, &text) {
                        Ok(()) => self.status = "Saved".into(),
                        Err(err) => self.status = format!("Saved, but search index failed: {err}"),
                    }
                }
                Err(err) => self.status = format!("Save failed: {err}"),
            }
        }
        // Looked for when the text is saved, not on every key press.
        self.refresh_named_here();
        self.report_set_aside();
    }

    /// The entries the open scene's text names that it is not linked to,
    /// directly or through what it is in, and not hidden for it: characters
    /// by any capitalized part of the name, as diagnostics find them; places,
    /// cultures, and lore by the whole name.
    pub fn named_in(&self, scene: Id, text: &str) -> Vec<Id> {
        let mentioned = self.link_index.get(&scene).cloned().unwrap_or_default();
        let notes: Vec<(Id, String)> = self
            .binder
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Note && node.deleted_at.is_none() && self.binder.is_in_notes(node.id))
            .filter(|node| !node.title.trim().is_empty())
            .filter(|node| !self.hidden_named.contains(&(scene, node.id)))
            .filter(|node| !mentioned.iter().any(|seen| seen.trim() == node.title.trim().to_lowercase()))
            .filter(|node| {
                let mut at = Some(scene);
                while let Some(id) = at {
                    if node.links.contains(&id) {
                        return false;
                    }
                    at = self.binder.get(id).and_then(|item| item.parent_id);
                }
                true
            })
            .map(|node| (node.id, node.title.clone()))
            .collect();
        let titles: Vec<String> = notes.iter().map(|(_, title)| title.clone()).collect();
        let found = mentions(text, &titles);
        notes.iter().filter(|(_, title)| found.iter().any(|mention| mention.name == *title)).map(|(id, _)| *id).collect()
    }

    /// Look again for what the open scene names, after it is opened or saved
    /// or a suggestion is taken.
    fn refresh_named_here(&mut self) {
        let scene = self.selected.filter(|id| self.binder.get(*id).is_some_and(|node| node.kind == NodeKind::Scene));
        self.named_here = match (scene, &self.session) {
            (Some(scene), Some(session)) => {
                let text = session.plain_text();
                self.named_in(scene, &text)
            }
            _ => Vec::new(),
        };
    }

    /// Link the open scene to an entry its text names.
    /// Link a note the open scene names to the scene.
    pub fn link_named(&mut self, note: Id) {
        if let Some(scene) = self.selected {
            self.set_note_link(note, scene, true);
            self.refresh_named_here();
        }
    }

    /// The notes that reach an item: those linked to it from Context or
    /// mentioned in its text (with whether the link is one to take away), and
    /// apart, those linked to what it is in, with what that is.
    pub fn notes_reaching(&self, item: Id) -> (Vec<(Id, String, bool)>, Vec<(Id, String, String)>) {
        let mentioned = self.link_index.get(&item).cloned().unwrap_or_default();
        let mut own = Vec::new();
        let mut through = Vec::new();
        for node in &self.binder.nodes {
            if node.kind != NodeKind::Note || node.deleted_at.is_some() || !self.binder.is_in_notes(node.id) || node.id == item {
                continue;
            }
            let linked = node.links.contains(&item);
            if linked || mentioned.iter().any(|seen| seen.trim() == node.title.trim().to_lowercase()) {
                own.push((node.id, node.title.clone(), linked));
                continue;
            }
            let mut up = self.binder.get(item).and_then(|found| found.parent_id);
            while let Some(id) = up {
                if node.links.contains(&id) {
                    let from = self.binder.get(id).map(|found| found.title.clone()).unwrap_or_default();
                    through.push((node.id, node.title.clone(), from));
                    break;
                }
                up = self.binder.get(id).and_then(|found| found.parent_id);
            }
        }
        (own, through)
    }

    /// A note, by its id, while it is in Notes and not in the trash.
    pub fn note_of(&self, id: Id) -> Option<&BinderNode> {
        self.binder
            .get(id)
            .filter(|node| node.kind == NodeKind::Note && node.deleted_at.is_none() && self.binder.is_in_notes(node.id))
    }

    /// Whether a note reaches an item: mentioned in its text, or linked to
    /// it or to what it is in.
    pub fn note_reaches(&self, note: Id, item: Id) -> bool {
        let Some(node) = self.note_of(note) else {
            return false;
        };
        let title = node.title.trim().to_lowercase();
        if self.link_index.get(&item).is_some_and(|seen| seen.iter().any(|seen| seen.trim() == title)) {
            return true;
        }
        let mut at = Some(item);
        while let Some(id) = at {
            if node.links.contains(&id) {
                return true;
            }
            at = self.binder.get(id).and_then(|found| found.parent_id);
        }
        false
    }

    /// Link the note of this title to an item, from the item's Context.
    pub fn link_note_titled(&mut self, item: Id, title: &str) {
        match self.note_titled(title) {
            Some(note) => {
                self.set_note_link(note, item, true);
                self.refresh_named_here();
            }
            None => self.status = format!("No note is called “{}”.", title.trim()),
        }
    }

    /// Stop suggesting an entry for the open scene, until the program closes.
    pub fn hide_named(&mut self, entry: Id) {
        if let Some(scene) = self.selected {
            self.hidden_named.push((scene, entry));
            self.refresh_named_here();
        }
    }

    fn project_root(&self) -> Option<PathBuf> {
        self.store.as_ref().map(|store| store.borrow().root.clone())
    }

    /// Where the project stands with its remote, or `None` when it is not
    /// connected for sync. Asks git, not the network.
    pub fn sync_status(&self) -> Option<SyncStatus> {
        let root = self.project_root()?;
        git::is_connected(&root).then(|| git::status(&root).ok()).flatten()
    }

    /// Whether the project is connected for sync. Cheap enough for every redraw.
    pub fn sync_connected(&self) -> bool {
        self.project_root().is_some_and(|root| git::is_connected(&root))
    }

    /// Connect the project to the repository at `sync_address`.
    pub fn connect_sync(&mut self) {
        let Some(root) = self.project_root() else {
            return;
        };
        self.save_all();
        self.status = match git::connect(&root, &self.sync_address) {
            Ok(()) => "Connected. Sync sends the project there.".into(),
            Err(err) => format!("Could not connect: {err}"),
        };
    }

    /// Save everything and start a sync. Returns the project folder for the
    /// sync to run on, or `None` when one is running or something is unsaved.
    pub fn begin_sync(&mut self) -> Option<PathBuf> {
        if self.syncing {
            return None;
        }
        let root = self.project_root()?;
        self.save_all();
        if self.has_unsaved() {
            self.status = "Not synced: something could not be saved first.".into();
            return None;
        }
        self.syncing = true;
        self.status = "Syncing…".into();
        Some(root)
    }

    /// Take in what a sync did. If changes came in, the project is opened
    /// again to show them, and each scene where both versions were kept gets
    /// the track-changes baseline that lists the other machine's paragraphs.
    pub fn finish_sync(&mut self, result: writing_studio_core::Result<SyncReport>) {
        self.syncing = false;
        let report = match result {
            Ok(report) => report,
            Err(err) => {
                self.status = format!("Sync stopped: {err}");
                return;
            }
        };
        if report.received {
            self.open_again(|state| {
                let Some(store) = &state.store else {
                    return;
                };
                for baseline in &report.baselines {
                    if let Err(err) = store.borrow().save_tracking_baseline(baseline) {
                        state.status = format!("Synced, but a merged scene could not be marked: {err}");
                    }
                }
            });
        }
        self.sync_conflicts = report.conflicts.iter().map(|conflict| conflict.what.clone()).collect();
        let what = match (report.sent, report.received) {
            (true, true) => "sent and received changes",
            (true, false) => "sent this machine's changes",
            (false, true) => "received changes",
            (false, false) => "nothing to send or receive",
        };
        self.status = match self.sync_conflicts.len() {
            0 => format!("Synced: {what}."),
            count => format!("Synced: {what}. {count} things were changed on both machines; History lists them."),
        };
    }

    /// When the window closes: save, and commit for the next sync. Nothing
    /// goes over the network.
    pub fn commit_on_close(&mut self) {
        self.save_all();
        if let Some(root) = self.project_root().filter(|root| git::is_connected(root)) {
            let _ = git::commit_all(&root);
        }
    }

    /// What was wrong with the project's files when it opened, one line each.
    pub fn open_problems(&self) -> Vec<String> {
        self.store
            .as_ref()
            .map(|store| store.borrow().problems().iter().map(ToString::to_string).collect())
            .unwrap_or_default()
    }

    /// Whether this item's file could not be read when the project opened.
    pub fn is_missing(&self, id: Id) -> bool {
        self.store.as_ref().is_some_and(|store| store.borrow().is_missing(id))
    }

    fn note_open_problems(&mut self) {
        let count = self.open_problems().len();
        if count > 0 {
            self.status = format!(
                "Opened, but {count} of the project's files needed attention. History lists what was found and done."
            );
        }
    }

    /// Tell the writer about files that were changed outside the program and
    /// kept under another name, so that a save did not write over them.
    fn report_set_aside(&mut self) {
        let Some(store) = &self.store else {
            return;
        };
        let set = store.borrow().take_set_aside();
        let Some(first) = set.first() else {
            return;
        };
        self.status = if set.len() == 1 {
            format!(
                "{} was changed outside the program. Your version was saved, and the other was kept as {}.",
                first.file, first.kept_as
            )
        } else {
            format!(
                "{} files were changed outside the program. Your versions were saved, and the others were kept beside them, the first as {}.",
                set.len(),
                first.kept_as
            )
        };
    }

    /// Open the project again from its files, keeping the mode and the
    /// selection. `before` runs once it is open, before the selection is
    /// shown again.
    fn open_again(&mut self, before: impl FnOnce(&mut Self)) {
        let (Some(root), selected, mode) = (self.project_root(), self.selected, self.mode) else {
            return;
        };
        self.open_project_at(root);
        before(self);
        self.mode = mode;
        if let Some(id) = selected.filter(|id| self.binder.get(*id).is_some()) {
            self.select_node(id);
        }
    }

    /// Take in files changed outside the program, by a text editor or by
    /// git. Called when the window comes back to the front, so the change is
    /// seen before the writer types over it. A list or another file changed
    /// opens the project again; otherwise only the open scene's file is read.
    /// With anything unsaved nothing is replaced: the next save keeps the
    /// writer's version and the other beside it.
    pub fn check_files_on_disk(&mut self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let others = store.borrow().others_changed_outside();
        match others {
            Ok(false) => {}
            Ok(true) if self.has_unsaved() => {
                self.status = "The binder or a note was changed outside the program. When you save, your version stays and the other is kept beside it.".into();
                return;
            }
            Ok(true) => {
                self.open_again(|_| {});
                if !self.status.starts_with("Could not") {
                    self.status = "The project was changed outside the program, and has been read again.".into();
                }
                return;
            }
            Err(err) => {
                self.status = format!("Could not check the project's files: {err}");
                return;
            }
        }
        self.check_open_scene_on_disk();
    }

    fn check_open_scene_on_disk(&mut self) {
        let (Some(store), Some(session)) = (self.store.clone(), self.session.as_ref()) else {
            return;
        };
        let id = session.document_id();
        let dirty = session.is_dirty();
        let changed = store.borrow().changed_outside(id);
        match changed {
            Ok(true) => {}
            Ok(false) => return,
            Err(err) => {
                self.status = format!("Could not check this scene's file: {err}");
                return;
            }
        }
        if dirty {
            self.status = "This scene's file was changed outside the program. When you save, your version stays the scene and the other is kept beside it.".into();
            return;
        }
        let reread = store.borrow().reread_scene(id);
        match reread {
            Ok(Some((node, beats, document))) => {
                if let Some(current) = self.binder.nodes.iter_mut().find(|item| item.id == node.id) {
                    current.title = node.title.clone();
                    current.synopsis = node.synopsis;
                    current.status = node.status;
                    current.label = node.label;
                    current.color = node.color;
                }
                if beats.is_empty() {
                    self.beats.remove(&node.id);
                } else {
                    self.beats.insert(node.id, beats);
                }
                let indexed = self
                    .manuscript_index
                    .borrow_mut()
                    .upsert(id, &document.to_plain_text());
                let session = EditorSession::new(document);
                self.sync_editor_from_session(&session);
                self.session = Some(session);
                self.status = match indexed {
                    Ok(()) => format!("“{}” was changed outside the program, and has been read again.", node.title),
                    Err(err) => format!("“{}” was read again, but search index failed: {err}", node.title),
                };
            }
            Ok(None) => {}
            Err(err) => self.status = format!("Could not read this scene again: {err}"),
        }
    }

    pub fn save_all(&mut self) {
        self.flush_editor();
        if self.binder_dirty && self.store.is_some() && !self.persist_binder() {
            return;
        }
        if self.plot_dirty && self.store.is_some() && !self.persist_plot_now() {
            return;
        }
        if self.beats_dirty && self.store.is_some() && !self.persist_beats_now() {
            return;
        }
        self.note_writing_activity();
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
        self.plot_dirty = false;
        self.beats_dirty = false;
        self.settings_dirty = false;
        self.last_edit = None;
        self.status = "All changes saved locally.".into();
        self.report_set_aside();
    }

    pub fn autosave_ready(&self) -> bool {
        if !self.is_open() {
            return false;
        }
        let wait = autosave_wait(
            self.meta
                .as_ref()
                .map(|meta| meta.settings.autosave_seconds)
                .unwrap_or(2),
        );
        autosave_due(self.has_unsaved(), self.last_edit, Instant::now(), wait)
    }

    pub fn writing_day_needs_roll(&self) -> bool {
        if !self.is_open() {
            return false;
        }
        let today = local_day();
        self.meta.as_ref().is_some_and(|meta| {
            !meta.settings.writing.day.is_empty() && meta.settings.writing.day != today
        })
    }

    pub fn session_label_due(&self) -> bool {
        self.is_open()
            && format_elapsed(self.session_elapsed_secs()) != format_elapsed(self.session_shown_secs)
    }

    pub fn note_session_label(&mut self) {
        self.session_shown_secs = self.session_elapsed_secs();
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

    pub fn writing_clock_tick(&mut self) {
        if self.writing_day_needs_roll() {
            self.begin_writing_session();
        }
    }

    /// Open a mode, leaving focus mode. Choosing the mode that is already
    /// open does nothing. Returns whether anything changed.
    pub fn choose_mode(&mut self, mode: WorkspaceMode) -> bool {
        if self.mode == mode && !self.focus_mode {
            return false;
        }
        self.mode = mode;
        self.focus_mode = false;
        self.open_menu = None;
        true
    }

    /// The container the scene list shows: the one chosen in the library
    /// while it is still a container in the manuscript or the trash, or
    /// else the chapter of the selected scene, or else the first book.
    pub fn shown_container(&self) -> Option<Id> {
        let trash = self.binder.trash_root().map(|node| node.id);
        let containers = self.binder.manuscript_containers();
        let is_container = |id: Id| Some(id) == trash || containers.iter().any(|(row, _)| *row == id);
        self.library_selection
            .filter(|id| is_container(*id))
            .or_else(|| {
                self.selected
                    .and_then(|id| self.binder.get(id))
                    .filter(|node| matches!(node.kind, NodeKind::Scene | NodeKind::Note))
                    .and_then(|node| node.parent_id)
                    .filter(|id| is_container(*id))
            })
            .or_else(|| containers.first().map(|(id, _)| *id))
    }

    pub fn is_collapsed(&self, id: Id) -> bool {
        self.meta
            .as_ref()
            .is_some_and(|meta| meta.settings.collapsed.contains(&id))
    }

    /// Fold an item shut, or open it again. It is this computer's choice,
    /// saved with the theme.
    pub fn toggle_collapsed(&mut self, id: Id) {
        let Some(meta) = self.meta.as_mut() else {
            return;
        };
        let collapsed = &mut meta.settings.collapsed;
        match collapsed.iter().position(|item| *item == id) {
            Some(at) => {
                collapsed.remove(at);
            }
            None => collapsed.push(id),
        }
        meta.touch();
        self.settings_dirty = true;
        self.note_edit();
    }

    /// The binder's rows: the manuscript's books, parts, chapters, folders,
    /// and the scenes in them, each at its depth, leaving out what is inside
    /// a folded one. Narrowed by a search, a saved filter, or a keyword
    /// chosen in the library, it holds the scenes that match and
    /// what they are in, unfolded.
    pub fn binder_rows(&self) -> Vec<BinderRow> {
        let mut rows = Vec::new();
        if let Some(draft) = self.binder.draft_root() {
            let shown = self.shown_scenes();
            self.push_rows(draft.id, 0, shown.as_ref(), &mut rows);
        }
        rows
    }

    /// The Notes' rows: its folders and notes, each at its depth, leaving out
    /// what is inside a folded folder. A search narrows them, as it does the
    /// manuscript: to the notes that match and the folders they are in.
    pub fn note_rows(&self) -> Vec<BinderRow> {
        let Some(root) = self.binder.notes_root() else {
            return Vec::new();
        };
        let query = self.search.trim().to_string();
        let hits = if query.is_empty() { HashSet::new() } else { self.manuscript_index.borrow().search(&query) };
        let mut rows = Vec::new();
        self.push_note_rows(root.id, 0, &query, &hits, &mut rows);
        rows
    }

    fn push_note_rows(&self, parent: Id, depth: u32, query: &str, hits: &HashSet<Id>, rows: &mut Vec<BinderRow>) {
        let in_notes = |node: &&BinderNode| matches!(node.kind, NodeKind::Folder | NodeKind::Note);
        let matches = |id: Id| binder_matches(&self.binder, id, query, hits);
        for node in self.binder.children(Some(parent)).into_iter().filter(in_notes) {
            if depth > 64 {
                continue;
            }
            let inside = self.items_inside(node.id);
            let visible = query.is_empty() || matches(node.id) || inside.iter().any(|id| matches(*id));
            if !visible {
                continue;
            }
            let has_children = self.binder.children(Some(node.id)).into_iter().any(|child| in_notes(&child));
            let collapsed = has_children && query.is_empty() && self.is_collapsed(node.id);
            rows.push(self.binder_row(node, depth, has_children, collapsed));
            if has_children && !collapsed {
                self.push_note_rows(node.id, depth + 1, query, hits, rows);
            }
        }
    }

    /// Everything inside an item, at any depth.
    fn items_inside(&self, id: Id) -> Vec<Id> {
        let mut found = Vec::new();
        let mut open = vec![id];
        while let Some(at) = open.pop() {
            for child in self.binder.children(Some(at)) {
                found.push(child.id);
                open.push(child.id);
            }
        }
        found
    }

    /// A note or a folder of notes, in the chosen folder of notes or at the
    /// top of Notes. It is chosen and opened in the page.
    pub fn add_in_notes(&mut self, kind: NodeKind) -> Option<Id> {
        let root = self.binder.notes_root()?.id;
        let parent = self
            .selected
            .filter(|id| self.binder.get(*id).is_some_and(|node| node.kind == NodeKind::Folder) && self.binder.is_in_notes(*id))
            .unwrap_or(root);
        self.flush_editor();
        let id = self.create_node(kind, Some(parent))?;
        self.select_node(id);
        self.mode = WorkspaceMode::Writing;
        Some(id)
    }

    /// The trash's contents, under its row when it is open.
    pub fn trash_rows(&self) -> Vec<BinderRow> {
        let Some(trash) = self.binder.trash_root() else {
            return Vec::new();
        };
        self.binder.children(Some(trash.id)).into_iter().map(|node| self.binder_row(node, 1, false, false)).collect()
    }

    fn push_rows(&self, parent: Id, depth: u32, shown: Option<&HashSet<Id>>, rows: &mut Vec<BinderRow>) {
        let in_binder = |node: &&BinderNode| {
            matches!(node.kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Folder | NodeKind::Scene)
        };
        let visible = |node: &BinderNode| match shown {
            None => true,
            Some(set) if node.kind == NodeKind::Scene => set.contains(&node.id),
            Some(set) => self.binder.scenes_under(node.id).iter().any(|(_, scene)| set.contains(scene)),
        };
        for node in self.binder.children(Some(parent)).into_iter().filter(in_binder) {
            if !visible(node) || depth > 64 {
                continue;
            }
            let has_children = self.binder.children(Some(node.id)).into_iter().filter(in_binder).any(|child| visible(child));
            let collapsed = has_children && shown.is_none() && self.is_collapsed(node.id);
            rows.push(self.binder_row(node, depth, has_children, collapsed));
            if has_children && !collapsed {
                self.push_rows(node.id, depth + 1, shown, rows);
            }
        }
    }

    fn binder_row(&self, node: &BinderNode, depth: u32, has_children: bool, collapsed: bool) -> BinderRow {
        let scene = node.kind == NodeKind::Scene;
        let held: Vec<Id> = if scene {
            Vec::new()
        } else {
            self.binder.scenes_under(node.id).into_iter().map(|(_, id)| id).collect()
        };
        let finished = held
            .iter()
            .filter_map(|id| self.binder.get(*id))
            .filter(|held| ["done", "final"].contains(&held.status.trim().to_lowercase().as_str()))
            .count();
        BinderRow {
            id: node.id,
            depth,
            kind: node.kind,
            title: node.title.clone(),
            has_children,
            collapsed,
            status: if scene { node.status.clone() } else { String::new() },
            keywords: self.keyword_colors_of(node),
            left_out: self.binder.is_left_out(node.id),
            scenes: held.len(),
            finished,
            compiles: (!node.kind.is_root() && self.binder.is_in_notes(node.id)).then_some(node.footnotes),
        }
    }

    /// The scenes the binder shows while it is narrowed, or `None` when it
    /// shows them all: a saved filter's matches from every book, those of a
    /// chosen keyword, and those the search finds (`#plot`
    /// and `-#plot` are keywords; a chapter whose name matches keeps its
    /// scenes).
    pub fn shown_scenes(&self) -> Option<HashSet<Id>> {
        let filter = self.shown_filter();
        let searching = !self.search.trim().is_empty();
        if filter.is_none() && !self.narrowed() && !searching {
            return None;
        }
        let scenes: Vec<Id> = match &filter {
            Some(filter) => self.filter_matches(filter).into_iter().map(|(_, scene)| scene).collect(),
            None => self
                .binder
                .draft_root()
                .map(|draft| self.binder.scenes_under(draft.id).into_iter().map(|(_, scene)| scene).collect())
                .unwrap_or_default(),
        };
        let query = KeywordQuery::parse(&self.search);
        let hits = self.manuscript_index.borrow().search(&query.text);
        let titled = |scene: Id| {
            !query.text.is_empty()
                && {
                    let mut at = self.binder.get(scene).and_then(|node| node.parent_id);
                    let mut found = false;
                    while let Some(node) = at.and_then(|id| self.binder.get(id)) {
                        if node.kind.is_root() {
                            break;
                        }
                        found |= writing_studio_search::matches_query(&node.title, &query.text);
                        at = node.parent_id;
                    }
                    found
                }
        };
        Some(
            scenes
                .into_iter()
                .filter(|scene| self.chosen_allows(*scene))
                .filter(|scene| self.binder.keywords_match(*scene, &query.with, false, &query.without))
                .filter(|scene| titled(*scene) || binder_matches(&self.binder, *scene, &query.text, &hits))
                .collect(),
        )
    }

    /// Show a library row's scenes. A row of the manuscript or the trash
    /// opens Writing when another mode is open.
    pub fn choose_library(&mut self, id: Id) {
        self.library_selection = Some(id);
        self.filter_selection = None;
        self.open_menu = None;
        self.focus_mode = false;
        // A book chosen here is the current book: its words and target are
        // the ones shown, and it is the one exported.
        if self.binder.get(id).is_some_and(|node| node.kind == NodeKind::Book)
            && self.compile_book_id() != Some(id)
        {
            self.set_compile_book(id);
        }
        if !matches!(self.mode, WorkspaceMode::Writing | WorkspaceMode::MindMap | WorkspaceMode::Outliner) {
            self.mode = WorkspaceMode::Writing;
        }
    }

    /// An item's keywords with each one's color as a hex value.
    pub fn keyword_colors_of(&self, node: &BinderNode) -> Vec<(String, String)> {
        node.keywords
            .iter()
            .map(|word| (word.clone(), binder_color_hex(self.binder.keyword_color(word)).to_string()))
            .collect()
    }

    /// The saved filter chosen in the library, while it still exists.
    pub fn shown_filter(&self) -> Option<SavedFilter> {
        let id = self.filter_selection?;
        self.binder.filters.iter().find(|filter| filter.id == id).cloned()
    }

    /// A filter's matching scenes, each with what holds it. The text is
    /// looked up in the search index, which every save keeps current.
    fn filter_matches(&self, filter: &SavedFilter) -> Vec<(Id, Id)> {
        let with_text = (!filter.text.trim().is_empty())
            .then(|| self.manuscript_index.borrow().search(&filter.text));
        // A note no longer there matches nothing, so the filter shows that
        // rather than everything.
        self.binder
            .scenes_matching(filter)
            .into_iter()
            .filter(|scene| filter.entries.iter().all(|note| self.note_reaches(*note, *scene)))
            .filter(|scene| {
                with_text.as_ref().is_none_or(|hits| {
                    self.binder
                        .get(*scene)
                        .and_then(|node| node.document_id)
                        .is_some_and(|document| hits.contains(&document))
                })
            })
            .filter_map(|scene| Some((self.binder.get(scene)?.parent_id?, scene)))
            .collect()
    }

    /// What a saved filter looks for, a phrase each: "plot or harbor",
    /// "not draft", "with Mara", "Revise", "“dawn”". Its default name and
    /// its row's tooltip.
    pub fn filter_parts(&self, filter: &SavedFilter) -> Vec<String> {
        let mut parts = Vec::new();
        if !filter.keywords.is_empty() {
            parts.push(filter.keywords.join(if filter.any { " or " } else { " and " }));
        }
        parts.extend(filter.without.iter().map(|word| format!("not {word}")));
        parts.extend(
            filter.entries.iter().filter_map(|id| self.note_of(*id)).map(|note| format!("with {}", note.title)),
        );
        if !filter.status.trim().is_empty() {
            parts.push(filter.status.trim().to_string());
        }
        if !filter.text.trim().is_empty() {
            parts.push(format!("“{}”", filter.text.trim()));
        }
        parts
    }

    /// Show a saved filter's matches in the scene list.
    pub fn choose_filter(&mut self, id: Id) {
        self.filter_selection = Some(id);
        self.open_menu = None;
        self.focus_mode = false;
        if !matches!(self.mode, WorkspaceMode::Writing | WorkspaceMode::History) {
            self.mode = WorkspaceMode::Writing;
        }
    }

    /// Keep a new saved filter, and show it. A filter with no name is named
    /// for what it looks for.
    pub fn add_filter(&mut self, mut filter: SavedFilter) {
        if filter.name.trim().is_empty() {
            let parts = self.filter_parts(&filter);
            filter.name = if parts.is_empty() { "All scenes".into() } else { parts.join(", ") };
        }
        let id = filter.id;
        let status = match self.binder.filters.iter_mut().find(|kept| kept.id == id) {
            Some(kept) => {
                *kept = filter;
                "Filter changed."
            }
            None => {
                self.binder.filters.push(filter);
                "Filter saved."
            }
        };
        self.editing_filter = None;
        self.open_menu = None;
        if self.persist_binder() {
            self.status = status.into();
        }
        self.choose_filter(id);
    }

    /// Open the filter form on a saved filter, to change it.
    pub fn edit_filter(&mut self, id: Id) {
        self.editing_filter = Some(id);
        self.open_menu = Some(FrameMenu::Filter);
    }

    /// Open the filter form for a new filter, or close it.
    pub fn toggle_new_filter(&mut self) {
        let open = self.open_menu == Some(FrameMenu::Filter) && self.editing_filter.is_none();
        self.editing_filter = None;
        self.open_menu = if open { None } else { Some(FrameMenu::Filter) };
    }

    pub fn remove_filter(&mut self, id: Id) {
        self.binder.filters.retain(|filter| filter.id != id);
        if self.filter_selection == Some(id) {
            self.filter_selection = None;
        }
        if self.persist_binder() {
            self.status = "Filter removed. The scenes stay as they are.".into();
        }
    }

    pub fn add_keyword(&mut self, id: Id, name: &str) {
        if !name.trim().is_empty() && tidy_keyword(name).is_none() {
            self.status = format!("“{}” is not a keyword: each part around / needs a name.", name.trim());
            return;
        }
        if self.binder.add_keyword(id, name) {
            self.persist_binder();
        }
    }

    pub fn remove_keyword(&mut self, id: Id, name: &str) {
        if self.binder.remove_keyword(id, name) {
            self.persist_binder();
        }
    }

    pub fn cycle_keyword_color(&mut self, name: &str) {
        if self.binder.cycle_keyword_color(name) {
            self.persist_binder();
        }
    }

    /// The library's keyword list, without the children of a folded one.
    pub fn keyword_list(&self) -> Vec<KeywordRow> {
        let folded = |row: &KeywordRow| {
            self.folded_keywords
                .iter()
                .any(|parent| row.name.to_lowercase().starts_with(&format!("{}/", parent.to_lowercase())))
        };
        self.binder.keyword_tree(self.keyword_sort).into_iter().filter(|row| !folded(row)).collect()
    }

    /// Fold or unfold a keyword's children in the library's list.
    pub fn toggle_keyword_fold(&mut self, name: &str) {
        let before = self.folded_keywords.len();
        self.folded_keywords.retain(|kept| !same_keyword(kept, name));
        if self.folded_keywords.len() == before {
            self.folded_keywords.push(name.to_string());
        }
    }

    /// Narrow the scene list, the mind map, and the outliner to a keyword
    /// and what is nested under it, as a search narrows them; choosing it
    /// again lets it go. Another mode gives way to the page.
    pub fn choose_keyword(&mut self, name: &str) {
        if self.keyword_filter.as_deref().is_some_and(|kept| same_keyword(kept, name)) {
            self.keyword_filter = None;
            return;
        }
        self.keyword_filter = Some(name.to_string());
        self.open_menu = None;
        if !matches!(self.mode, WorkspaceMode::Writing | WorkspaceMode::MindMap | WorkspaceMode::Outliner) {
            self.mode = WorkspaceMode::Writing;
        }
    }

    /// Rename a keyword on every item that carries it, its children and
    /// saved filters with it. A name already in use merges the two, after a
    /// second Enter. The status line offers Undo.
    pub fn rename_keyword(&mut self, old: &str, typed: &str) {
        let Some(new) = tidy_keyword(typed) else {
            self.status = format!("“{}” is not a keyword: each part around / needs a name.", typed.trim());
            return;
        };
        if new == old.trim() {
            self.renaming_keyword = None;
            return;
        }
        let count = self.binder.items_with_keyword(old);
        let items = if count == 1 { "1 item".to_string() } else { format!("{count} items") };
        let merges = self.binder.keyword_merges(old, &new);
        let waiting = self.confirm_keyword_merge.as_ref().is_some_and(|(from, to)| from == old && *to == new);
        if merges && !waiting {
            self.confirm_keyword_merge = Some((old.to_string(), new.clone()));
            self.status = format!("“{new}” is in use: “{old}” joins it on {items}. Press Enter again to merge them.");
            return;
        }
        self.confirm_keyword_merge = None;
        let snapshot = KeywordSnapshot {
            items: self
                .binder
                .nodes
                .iter()
                .filter(|node| !node.keywords.is_empty())
                .map(|node| (node.id, node.keywords.clone()))
                .collect(),
            keywords: self.binder.keywords.clone(),
            filters: self.binder.filters.clone(),
            chosen: self.keyword_filter.clone(),
        };
        match self.binder.rename_keyword(old, &new) {
            Err(why) => self.status = why,
            Ok(_) => {
                if !self.persist_binder() {
                    return;
                }
                self.renaming_keyword = None;
                if self.keyword_filter.as_deref().is_some_and(|chosen| keyword_covers(old, chosen)) {
                    self.keyword_filter = Some(new.clone());
                }
                self.status = if merges {
                    format!("Merged “{old}” into “{new}” on {items}.")
                } else {
                    format!("Renamed “{old}” to “{new}” on {items}.")
                };
                self.keyword_undo = Some(snapshot);
                self.offer_undo(UndoOffer::Keywords);
            }
        }
    }

    /// What the mind map has in its middle: the book the series menu
    /// chose, the only book, or the whole manuscript when it holds several.
    pub fn map_root(&self) -> Option<Id> {
        if let Some(book) = self.series_scope() {
            return Some(book);
        }
        match self.binder.manuscript_books().as_slice() {
            [book] => Some(book.id),
            _ => self.binder.draft_root().map(|draft| draft.id),
        }
    }

    /// Fold or unfold a branch of the mind map.
    pub fn toggle_map_fold(&mut self, id: Id) {
        if let Some(at) = self.folded_map.iter().position(|kept| *kept == id) {
            self.folded_map.remove(at);
        } else {
            self.folded_map.push(id);
        }
    }

    /// Zoom the mind map in or out from where it is, a fitted map from
    /// its natural size; `None` fits it to the window again.
    pub fn zoom_map(&mut self, by: Option<f64>) {
        self.map_zoom = by.map(|by| (self.map_zoom.unwrap_or(1.0) * by).clamp(0.25, 3.0));
    }

    /// The ideas the mind map shows besides the binder, each with the node
    /// it hangs from.
    pub fn map_extras(&self) -> Vec<crate::mindmap::Extra> {
        use crate::mindmap::{Extra, MapKind};
        self.mindmap
            .ideas
            .iter()
            .map(|idea| Extra { id: idea.id, kind: MapKind::Idea, joined: idea.attached.into_iter().collect() })
            .collect()
    }

    /// The whole mind map as it is drawn: the binder from `root`, ideas,
    /// and the nodes placed by hand.
    pub fn map_layout(&self, root: Id) -> crate::mindmap::MapLayout {
        let extras = self.map_extras();
        let placed: Vec<(Id, f64, f64)> = self.mindmap.positions.iter().map(|kept| (kept.id, kept.x, kept.y)).collect();
        crate::mindmap::layout_all(&self.binder, root, &self.folded_map, &extras, &placed)
    }

    fn persist_mindmap(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_mindmap(&self.mindmap) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save the mind map: {err}");
                false
            }
        }
    }

    /// Change the mind map's own things and save them.
    fn edit_map(&mut self, change: impl FnOnce(&mut MindMap) -> bool) -> bool {
        let changed = change(&mut self.mindmap);
        changed && self.persist_mindmap()
    }

    /// Choose a node on the map: a binder node is opened in the inspector
    /// as anywhere; an idea is the map's own choice. While a
    /// line is being drawn, the node is where it ends.
    pub fn choose_on_map(&mut self, id: Id) {
        if let Some(from) = self.map_join.take() {
            if from != id && self.edit_map(|map| map.add_line(from, id).is_some()) {
                self.status = "Joined. Write a word on the line under the map if it needs one.".into();
            }
            return;
        }
        if self.binder.get(id).is_some() {
            self.map_focus = None;
            self.select_node(id);
        } else {
            self.map_focus = Some(id);
        }
    }

    /// The node chosen on the map, idea or entry first.
    pub fn map_chosen(&self) -> Option<Id> {
        self.map_focus.or(self.selected)
    }

    /// Start drawing a line from a node; the next node clicked ends it.
    pub fn start_join(&mut self, from: Id) {
        self.map_join = Some(from);
        self.status = "Click the node to join it to. Escape lets it go.".into();
    }

    /// A new idea, hanging from the chosen node (or loose), chosen and
    /// named next.
    pub fn add_idea(&mut self, attached: Option<Id>) -> Id {
        let id = self.mindmap.add_idea(attached);
        self.persist_mindmap();
        self.map_focus = Some(id);
        self.status = "Added an idea. Name it under the map.".into();
        id
    }

    pub fn set_idea_title(&mut self, id: Id, title: String) {
        self.edit_map(|map| match map.ideas.iter_mut().find(|idea| idea.id == id) {
            Some(idea) if idea.title != title => {
                idea.title = title;
                true
            }
            _ => false,
        });
    }

    pub fn set_idea_note(&mut self, id: Id, note: String) {
        self.edit_map(|map| match map.ideas.iter_mut().find(|idea| idea.id == id) {
            Some(idea) if idea.note != note => {
                idea.note = note;
                true
            }
            _ => false,
        });
    }

    pub fn remove_idea(&mut self, id: Id) {
        if self.edit_map(|map| map.remove_idea(id)) {
            self.map_focus = None;
            self.status = "Idea removed.".into();
        }
    }

    pub fn set_line_label(&mut self, id: Id, label: String) {
        self.edit_map(|map| match map.lines.iter_mut().find(|line| line.id == id) {
            Some(line) if line.label != label => {
                line.label = label;
                true
            }
            _ => false,
        });
    }

    pub fn remove_line(&mut self, id: Id) {
        self.edit_map(|map| {
            let before = map.lines.len();
            map.lines.retain(|line| line.id != id);
            map.lines.len() != before
        });
    }

    /// Keep a node where the writer dropped it, in map units.
    pub fn place_on_map(&mut self, id: Id, x: f64, y: f64) {
        self.edit_map(|map| {
            map.place(id, x, y);
            true
        });
    }

    /// Put every node back where the map lays it out.
    pub fn tidy_map(&mut self) {
        if self.edit_map(|map| {
            let had = !map.positions.is_empty();
            map.positions.clear();
            had
        }) {
            self.status = "The map is laid out again.".into();
        }
    }

    /// Make an idea a scene: in the chapter or folder it hangs from (or the
    /// one it is dropped on), after the scene it hangs from, or where a new
    /// scene would go. Its title is the scene's, its note the synopsis, and
    /// its lines go with it.
    pub fn idea_to_scene(&mut self, idea: Id, into: Option<Id>) -> Option<Id> {
        let found = self.mindmap.idea(idea)?.clone();
        let anchor = into.or(found.attached).and_then(|id| self.binder.get(id)).cloned();
        let (parent, after) = match anchor {
            Some(node) if self.binder.accepts(node.id, NodeKind::Scene) => {
                (Some(node.id), self.binder.children(Some(node.id)).last().map(|last| last.id))
            }
            Some(node) if node.kind == NodeKind::Scene => (node.parent_id, Some(node.id)),
            _ => (self.infer_parent(NodeKind::Scene), None),
        };
        self.flush_editor();
        let id = self.create_node(NodeKind::Scene, parent)?;
        if let Some(parent) = parent {
            let _ = self.binder.put_after(id, parent, after);
        }
        if let Some(node) = self.binder.get_mut(id) {
            if !found.title.trim().is_empty() {
                node.title = found.title.trim().to_string();
            }
            node.synopsis = found.note.trim().to_string();
        }
        self.persist_binder();
        self.mindmap.idea_became(idea, id);
        self.persist_mindmap();
        self.map_focus = None;
        self.select_node(id);
        self.status = format!("“{}” is a scene now.", found.title.trim());
        self.offer_undo(UndoOffer::Added { id });
        Some(id)
    }

    /// Whether what is dragged on the map can be let go on `target`: a
    /// binder node where the binder allows it, an idea on any node (onto a
    /// chapter or folder it becomes a scene there).
    pub fn map_drop_allowed(&self, dragged: Id, target: Id) -> bool {
        if dragged == target {
            return false;
        }
        if self.binder.get(dragged).is_some() {
            return self.binder.drop_allowed_at(dragged, target, self.drop_place);
        }
        if self.mindmap.idea(dragged).is_some() {
            return true;
        }
        false
    }

    /// Let go of what is dragged on the map over `target`: a binder node
    /// moves there; an idea becomes a scene in a chapter or folder, or hangs
    /// from anything else.
    pub fn drop_on_map(&mut self, target: Id) {
        let Some(dragged) = self.dragging else {
            return;
        };
        if !self.map_drop_allowed(dragged, target) {
            self.end_drag();
            return;
        }
        if self.binder.get(dragged).is_some() {
            self.drop_in_binder(target);
            return;
        }
        self.end_drag();
        if self.mindmap.idea(dragged).is_some() {
            if self.binder.accepts(target, NodeKind::Scene) {
                self.idea_to_scene(dragged, Some(target));
            } else if self.edit_map(|map| match map.ideas.iter_mut().find(|idea| idea.id == dragged) {
                Some(idea) => {
                    idea.attached = Some(target);
                    true
                }
                None => false,
            }) {
                self.status = "The idea hangs from there now.".into();
            }
        }
    }

    /// The mind map as drawn now, the book's or the notes', with its middle.
    pub fn current_map(&self) -> Option<(Id, crate::mindmap::MapLayout)> {
        // The notes view: the Notes root and what is under it, on the same
        // map as the book.
        if self.map_notes {
            let root = self.binder.notes_root()?.id;
            return Some((root, self.map_layout(root)));
        }
        let root = self.map_root()?;
        Some((root, self.map_layout(root)))
    }

    /// Let go of the chosen keyword. Returns whether one was chosen.
    pub fn clear_keyword_filter(&mut self) -> bool {
        self.keyword_filter.take().is_some()
    }

    /// Whether the scene list, the mind map, and the outliner are narrowed
    /// to a keyword chosen in the library.
    pub fn narrowed(&self) -> bool {
        self.keyword_filter.is_some()
    }

    /// Whether a scene shows while they are narrowed: it carries the chosen
    /// keyword, or one nested under it, itself or through the chapter, part,
    /// or book it is in, as a saved filter matches; and the chosen story
    /// entry reaches it.
    pub fn chosen_allows(&self, id: Id) -> bool {
        self.keyword_filter
            .as_deref()
            .is_none_or(|wanted| self.binder.keywords_through(id).iter().any(|word| keyword_covers(wanted, word)))
    }

    /// The open scene's context: the scene, where it sits, its plot points,
    /// and the entries that reach it.
    pub fn scene_context(&self, scene: Id) -> Option<writing_studio_context::SceneContext> {
        let sources = writing_studio_context::Sources {
            binder: &self.binder,
            plot: &self.plot,
            note_text: &|id| self.item_text(id),
            mentions: &|id| self.link_index.get(&id).cloned().unwrap_or_default(),
        };
        writing_studio_context::scene_context(&sources, scene)
    }

    /// A chapter's, part's, book's, or folder's context: its own details,
    /// the synopses of what it holds, and the plot points and entries in it.
    pub fn level_context(&self, id: Id) -> Option<writing_studio_context::LevelContext> {
        let sources = writing_studio_context::Sources {
            binder: &self.binder,
            plot: &self.plot,
            note_text: &|id| self.item_text(id),
            mentions: &|id| self.link_index.get(&id).cloned().unwrap_or_default(),
        };
        let mut context = writing_studio_context::level_context(&sources, id)?;
        // A folder's own text is its first context.
        if context.kind == NodeKind::Folder {
            context.about = self.item_text(id).trim().to_string();
        }
        Some(context)
    }

    /// The plain text of an item with text: the open one as it is on the page,
    /// any other as saved.
    fn item_text(&self, id: Id) -> String {
        let Some(document_id) = self.binder.get(id).and_then(|node| node.document_id) else {
            return String::new();
        };
        if let Some(session) = self.session.as_ref().filter(|session| session.document_id() == document_id) {
            return session.plain_text();
        }
        self.store
            .as_ref()
            .and_then(|store| store.borrow().load_document(document_id).ok())
            .map(|document| document.to_plain_text())
            .unwrap_or_default()
    }

    /// The note templates in the project's `templates/`: each file's name and
    /// the title a note made from it starts with.
    pub fn note_templates(&self) -> Vec<(String, String)> {
        self.store
            .as_ref()
            .map(|store| store.borrow().note_templates())
            .unwrap_or_default()
            .into_iter()
            .map(|(name, file)| (name, file.header.title))
            .collect()
    }

    /// A new note in `folder`, a copy of the template in `templates/<name>`:
    /// its title, keywords, and text. It is chosen and opened in the page.
    pub fn new_note_from_template(&mut self, folder: Id, name: &str) -> Option<Id> {
        let store = self.store.clone()?;
        let template = store.borrow().note_templates().into_iter().find(|(file, _)| file == name).map(|(_, file)| file);
        let Some(template) = template else {
            self.status = format!("The template {name} is no longer in templates/.");
            return None;
        };
        self.flush_editor();
        let id = self.create_node(NodeKind::Note, Some(folder))?;
        let title = if template.header.title.trim().is_empty() { "New note".to_string() } else { template.header.title.clone() };
        self.edit_node(id, |node| node.title = title.clone());
        for word in template.header.other.list("tags").map(<[String]>::to_vec).unwrap_or_default() {
            self.binder.add_keyword(id, &word);
        }
        self.persist_binder();
        self.select_node(id);
        let text = writing_studio_format::blocks_to_markdown(&template.blocks);
        self.apply_editor_markdown(&text);
        self.flush_editor();
        self.status = format!("Made “{title}” from a template. Rename it, then write.");
        Some(id)
    }

    /// Keep a note as a template in `templates/`: its title, keywords, and
    /// text, for new notes to start from.
    pub fn save_note_as_template(&mut self, id: Id) {
        self.flush_editor();
        let (Some(store), Some(node)) = (self.store.clone(), self.binder.get(id).cloned()) else {
            return;
        };
        let blocks = node
            .document_id
            .and_then(|document_id| store.borrow().load_document(document_id).ok())
            .map(|document| document.blocks)
            .unwrap_or_default();
        let mut other = writing_studio_format::Header::default();
        other.set_list("tags", &node.keywords);
        let file = writing_studio_format::SceneFile {
            header: writing_studio_format::SceneHeader { title: node.title.clone(), other, ..Default::default() },
            blocks,
        };
        let saved = store.borrow().save_note_template(&file);
        self.status = match saved {
            Ok(name) => format!("“{}” is a template now, in templates/{name}.", node.title),
            Err(err) => format!("Could not keep it as a template: {err}"),
        };
    }

    /// Copy a file into the project for the open note, and give the Markdown
    /// that links to it from the note's file in `notes/`.
    pub fn attach_file_to_note(&mut self, id: Id, source: PathBuf) -> Option<String> {
        let store = self.store.clone()?;
        let name = source.file_name().and_then(|name| name.to_str()).unwrap_or("file").to_string();
        let copied = store.borrow().attach_file(id, &source);
        match copied {
            Ok(path) => {
                self.status = format!("{name} copied into attachments/.");
                let label: String = name.chars().filter(|ch| !matches!(ch, '[' | ']' | '\\')).collect();
                Some(format!("[{label}](../{path})"))
            }
            Err(err) => {
                self.status = format!("Could not attach {name}: {err}");
                None
            }
        }
    }

    /// Open a menu, or close it when it is the one already open.
    pub fn toggle_menu(&mut self, menu: FrameMenu) {
        self.open_menu = if self.open_menu == Some(menu) { None } else { Some(menu) };
    }

    /// Show a folded panel over the page, or close it when it is showing.
    pub fn toggle_overlay(&mut self, panel: FoldedPanel) {
        self.open_menu = None;
        self.overlay = if self.overlay == Some(panel) { None } else { Some(panel) };
    }

    /// Close the shortcuts window, or else the open menu, or else the panel
    /// shown over the page. Escape does this before anything else. Returns
    /// whether one was open.
    pub fn close_popup(&mut self) -> bool {
        std::mem::take(&mut self.show_shortcuts)
            || std::mem::take(&mut self.quick_open)
            || std::mem::take(&mut self.compile_open)
            || self.item_menu.take().is_some()
            || self.open_menu.take().is_some()
            || self.overlay.take().is_some()
    }

    /// Cancel a delete or restore that is waiting for its second click.
    /// Returns whether one was waiting.
    pub fn cancel_confirmation(&mut self) -> bool {
        let waiting = self.confirm_plotline.take().is_some()
            | self.confirm_backup.take().is_some()
            | self.confirm_kept.take().is_some()
            | std::mem::take(&mut self.confirm_empty_trash)
            | self.confirm_keyword_merge.take().is_some()
            | self.map_join.take().is_some();
        if waiting {
            self.status = "Cancelled.".into();
        }
        waiting
    }

    pub fn has_unsaved(&self) -> bool {
        self.settings_dirty
            || self.binder_dirty
            || self.plot_dirty
            || self.beats_dirty
            || self.session.as_ref().is_some_and(EditorSession::is_dirty)
    }

    fn note_edit(&mut self) {
        self.last_edit = Some(Instant::now());
    }

    fn begin_writing_session(&mut self) {
        if self.session.as_ref().is_some_and(EditorSession::is_dirty) {
            self.flush_editor();
        }
        let counts = self.book_outline_counts(false);
        let now = Timestamp::now().to_rfc3339();
        let today = local_day();
        let Some(meta) = &mut self.meta else {
            return;
        };
        meta.settings.writing.open_books(&today, &now, &counts);
        meta.touch();
        let meta = meta.clone();
        self.session_baseline = self
            .compile_book_id()
            .and_then(|book| counts.iter().find(|(id, _)| *id == book).map(|(_, words)| *words))
            .unwrap_or(0);
        self.session_started = Some(Instant::now());
        self.session_shown_secs = 0;
        let Some(store) = &self.store else {
            return;
        };
        if let Err(err) = store.borrow().persist_meta(&meta) {
            self.status = format!("Could not save writing statistics: {err}");
        }
    }

    fn note_writing_activity(&mut self) {
        let Some(meta) = &mut self.meta else {
            return;
        };
        if meta.settings.writing.session_started_at.is_empty() {
            return;
        }
        meta.settings.writing.note_activity(&Timestamp::now().to_rfc3339());
    }

    pub fn content_hits(&self) -> HashSet<Id> {
        // Only the words: `#plot` is a keyword, matched on the binder.
        self.manuscript_index.borrow().search(&KeywordQuery::parse(&self.search).text)
    }

    fn reindex_manuscript(&mut self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let Ok(documents) = store.borrow().load_all_documents() else {
            return;
        };
        let owned: Vec<(Id, String)> = documents
            .iter()
            .map(|document| (document.id, document.to_plain_text()))
            .collect();
        let pairs: Vec<(Id, &str)> = owned.iter().map(|(id, text)| (*id, text.as_str())).collect();
        if let Err(err) = self.manuscript_index.borrow_mut().replace_all(&pairs) {
            self.status = format!("Search index failed: {err}");
        }
        self.link_index.clear();
        self.openings.clear();
        for (id, text) in &owned {
            self.index_links(*id, text);
        }
    }

    /// Note what `text`, the text of item `id`, links to, and its first line.
    fn index_links(&mut self, id: Id, text: &str) {
        let titles: Vec<String> = writing_studio_format::wikilinks(text)
            .into_iter()
            .map(|link| link.title.to_lowercase())
            .collect();
        if titles.is_empty() {
            self.link_index.remove(&id);
        } else {
            self.link_index.insert(id, titles);
        }
        let first = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
        let opening: String = first.chars().take(160).collect();
        self.openings.insert(id, opening);
    }

    fn note_session_for_search(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_dirty() {
            return;
        }
        let id = session.document_id();
        let text = session.plain_text();
        self.index_links(id, &text);
        self.manuscript_index.borrow_mut().set_pending(id, text);
    }

    /// The note with this title, under Notes and not in the trash.
    pub fn note_titled(&self, title: &str) -> Option<Id> {
        self.binder
            .nodes
            .iter()
            .find(|node| {
                node.kind == NodeKind::Note
                    && node.deleted_at.is_none()
                    && self.binder.is_in_notes(node.id)
                    && writing_studio_format::same_title(&node.title, title)
            })
            .map(|node| node.id)
    }

    /// Every note a `[[link]]` can point at, with what a link to it shows
    /// as its tip: where it is, and its first line.
    pub fn note_link_targets(&self) -> Vec<(String, String)> {
        let mut targets: Vec<(String, String)> = self
            .binder
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Note && node.deleted_at.is_none() && self.binder.is_in_notes(node.id))
            .map(|node| {
                let folder = node.parent_id.and_then(|id| self.binder.get(id)).map(|up| up.title.clone()).unwrap_or_default();
                let opening = self.openings.get(&node.id).cloned().unwrap_or_default();
                let tip = if opening.is_empty() { folder } else { format!("{folder}: {opening}") };
                (node.title.clone(), tip)
            })
            .collect();
        targets.sort_by_key(|(title, _)| title.to_lowercase());
        targets
    }

    /// Open the note a `[[link]]` names, or say there is none.
    pub fn open_note_titled(&mut self, title: &str) {
        match self.note_titled(title) {
            Some(id) => {
                self.flush_editor();
                self.select_node(id);
                self.mode = WorkspaceMode::Writing;
            }
            None => self.status = format!("No note is called “{}”. Make one under Notes, or check the spelling.", title.trim()),
        }
    }

    /// The scenes, chapters, and notes whose text links to `note` with
    /// `[[…]]`, in binder order.
    pub fn mentions_of(&self, note: Id) -> Vec<Id> {
        let Some(title) = self.binder.get(note).map(|node| node.title.to_lowercase()) else {
            return Vec::new();
        };
        self.binder
            .nodes
            .iter()
            .filter(|node| node.id != note && node.deleted_at.is_none())
            .filter(|node| self.link_index.get(&node.id).is_some_and(|titles| titles.iter().any(|seen| seen.trim() == title.trim())))
            .map(|node| node.id)
            .collect()
    }

    /// Link a note to a binder item from Context, or take the link away.
    pub fn set_note_link(&mut self, note: Id, item: Id, linked: bool) {
        self.edit_node(note, |node| {
            node.links.retain(|id| *id != item);
            if linked {
                node.links.push(item);
            }
        });
        self.save_all();
        self.refresh_named_here();
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

    /// Show what changed between a snapshot and the current text. Asking for
    /// the snapshot that is already open closes the comparison.
    pub fn compare_snapshot(&mut self, snapshot_id: Id) {
        if self.compare_snapshot == Some(snapshot_id) {
            self.close_comparison();
            return;
        }
        self.compare_snapshot = Some(snapshot_id);
        self.compare_against = None;
        self.compare_revision = None;
    }

    /// The earlier synced versions of the selected scene, newest first.
    /// Empty when the project is not synced.
    pub fn scene_revisions(&self) -> Vec<Revision> {
        let (Some(root), Some(path)) = (self.project_root(), self.selected_file()) else {
            return Vec::new();
        };
        if !git::is_connected(&root) {
            return Vec::new();
        }
        git::revisions(&root, &path).unwrap_or_default()
    }

    fn selected_file(&self) -> Option<String> {
        let id = self.selected_node()?.document_id?;
        self.store.as_ref()?.borrow().file_of(id)
    }

    /// The selected scene as it was in an earlier version.
    fn revision_document(&self, earlier: &Earlier) -> writing_studio_core::Result<Document> {
        let missing = || writing_studio_core::Error::message("no scene is selected");
        let (Some(root), Some(store)) = (self.project_root(), self.store.as_ref()) else {
            return Err(missing());
        };
        let path = self.selected_file().ok_or_else(missing)?;
        let id = self.selected_node().and_then(|node| node.document_id).ok_or_else(missing)?;
        let html = match earlier {
            Earlier::Sync(commit) => git::file_at(&root, commit, &path)?,
            Earlier::Kept(kept_as) => store.borrow().read_kept_file(kept_as)?,
        };
        Ok(scene_document(&html, id))
    }

    /// Compare the selected scene with an earlier version. Asking for the
    /// one already open closes the comparison.
    pub fn compare_with_revision(&mut self, earlier: Earlier) {
        if self.compare_revision.as_ref() == Some(&earlier) {
            self.close_comparison();
            return;
        }
        self.compare_snapshot = None;
        self.compare_against = None;
        self.compare_revision = Some(earlier);
    }

    /// Put the selected scene's text back as it was in an earlier version.
    /// One undo step, so Ctrl+Z brings the current text back.
    pub fn restore_revision(&mut self, earlier: &Earlier) {
        let status = match earlier {
            Earlier::Sync(_) => "Restored the scene as it was at that sync. Undo brings the current text back.",
            Earlier::Kept(_) => "Restored the version changed outside the program. Undo brings the current text back.",
        };
        match self.revision_document(earlier) {
            Ok(found) => self.edit_text(status, |document| {
                document.blocks = found.blocks;
                true
            }),
            Err(err) => self.status = format!("Could not restore that version: {err}"),
        }
    }

    /// Files kept beside a project file because it was changed outside the
    /// program, newest first, each with whether it is the selected scene's.
    pub fn kept_files(&self) -> Vec<(SetAside, bool)> {
        let Some(store) = &self.store else {
            return Vec::new();
        };
        let selected = self.selected_file();
        let kept = store.borrow().kept_files().unwrap_or_default();
        kept.into_iter()
            .map(|item| {
                let mine = selected.as_ref() == Some(&item.file);
                (item, mine)
            })
            .collect()
    }

    /// Delete a kept file. The first click asks; the second deletes.
    pub fn request_kept_removal(&mut self, kept_as: String) {
        if self.confirm_kept.as_ref() != Some(&kept_as) {
            self.confirm_kept = Some(kept_as);
            self.status = "Click delete again to delete the kept file.".into();
            return;
        }
        self.confirm_kept = None;
        if self.compare_revision == Some(Earlier::Kept(kept_as.clone())) {
            self.close_comparison();
        }
        let Some(store) = &self.store else {
            return;
        };
        let removed = store.borrow().remove_kept_file(&kept_as);
        self.status = match removed {
            Ok(()) => format!("Deleted {kept_as}."),
            Err(err) => format!("Could not delete {kept_as}: {err}"),
        };
    }

    /// Compare the open snapshot with another snapshot of the same document.
    /// `None` compares it with the current text.
    pub fn set_compare_against(&mut self, other: Option<Id>) {
        self.compare_against = other.filter(|id| Some(*id) != self.compare_snapshot);
    }

    pub fn close_comparison(&mut self) {
        self.compare_snapshot = None;
        self.compare_against = None;
        self.compare_revision = None;
    }

    /// Nothing is written: both sides are read, and the live document and the
    /// snapshots stay as they are.
    pub fn snapshot_comparison(&self) -> Option<Result<SnapshotComparison, String>> {
        if let Some(earlier) = &self.compare_revision {
            let doc_id = self.selected_node().and_then(|node| node.document_id)?;
            let label = match earlier {
                Earlier::Sync(commit) => self
                    .scene_revisions()
                    .into_iter()
                    .find(|revision| &revision.commit == commit)
                    .map(|revision| format!("Synced {}", revision.date))
                    .unwrap_or_else(|| "An earlier sync".into()),
                Earlier::Kept(_) => "Changed outside".into(),
            };
            let result = self.revision_document(earlier).and_then(|old| {
                self.current_document(doc_id).map(|new| SnapshotComparison {
                    older: label,
                    newer: "Current text".into(),
                    diff: compare_documents(&old, &new),
                })
            });
            return Some(result.map_err(|err| err.to_string()));
        }
        let chosen = self.compare_snapshot?;
        let store = self.store.as_ref()?;
        let doc_id = self.selected_node().and_then(|node| node.document_id)?;
        let snapshots = store.borrow().list_snapshots(doc_id).ok()?;
        // The list is newest first.
        let chosen_at = snapshots.iter().position(|item| item.id == chosen)?;
        let other_at = self
            .compare_against
            .and_then(|id| snapshots.iter().position(|item| item.id == id));
        let name = |index: usize| {
            let item = &snapshots[index];
            format!("{} · {}", item.label, item.created_label())
        };
        let load = |index: usize| store.borrow().load_snapshot(snapshots[index].id);

        let result = match other_at {
            Some(other_at) => {
                let (old_at, new_at) = (chosen_at.max(other_at), chosen_at.min(other_at));
                load(old_at).and_then(|old| {
                    load(new_at).map(|new| SnapshotComparison {
                        older: name(old_at),
                        newer: name(new_at),
                        diff: compare_documents(&old, &new),
                    })
                })
            }
            None => load(chosen_at).and_then(|old| {
                self.current_document(doc_id).map(|new| SnapshotComparison {
                    older: name(chosen_at),
                    newer: "Current text".into(),
                    diff: compare_documents(&old, &new),
                })
            }),
        };
        Some(result.map_err(|err| err.to_string()))
    }

    /// The open editing session when it holds this document, so text that is
    /// not saved yet is still compared. Otherwise the saved document.
    fn current_document(&self, doc_id: Id) -> writing_studio_core::Result<Document> {
        if let Some(session) = self.session.as_ref().filter(|s| s.document_id() == doc_id) {
            return Ok(session.document().clone());
        }
        match &self.store {
            Some(store) => store.borrow().load_document(doc_id),
            None => Err(writing_studio_core::Error::message("no project is open")),
        }
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
        let restored = store.borrow().restore_snapshot(snapshot_id);
        self.show_restored(restored, "Snapshot");
    }

    /// Put back the selected scene's text from a backup; the rest of the
    /// project stays as it is.
    pub fn restore_scene_from_backup(&mut self, path: PathBuf) {
        self.flush_editor();
        let (Some(store), Some(id)) = (&self.store, self.selected_node().and_then(|node| node.document_id)) else {
            return;
        };
        let restored = store.borrow().restore_scene_from_backup(&path, id);
        self.show_restored(restored, "Scene");
    }

    fn show_restored(&mut self, restored: writing_studio_core::Result<Document>, what: &str) {
        match restored {
            Ok(document) => {
                let id = document.id;
                let text = document.to_plain_text();
                let indexed = self.manuscript_index.borrow_mut().upsert(id, &text);
                let session = EditorSession::new(document);
                self.sync_editor_from_session(&session);
                self.session = Some(session);
                self.status = match indexed {
                    Ok(()) => format!("{what} restored locally. Previous text was saved as a pre-restore snapshot."),
                    Err(err) => format!("{what} restored, but search index failed: {err}"),
                };
            }
            Err(err) => self.status = format!("Could not restore {}: {err}", what.to_lowercase()),
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
        let (meta, binder, plot) = {
            let store = store.borrow();
            (
                store.load_meta(),
                store.load_binder(),
                store.load_plot(),
            )
        };
        match (meta, binder, plot) {
            (Ok(meta), Ok(binder), Ok(plot)) => {
                let previous = self.selected;
                self.meta = Some(meta);
                self.binder = binder;
                self.plot = plot;
                self.plot_dirty = false;
                self.confirm_plotline = None;
                self.session = None;
                self.editor_text.clear();
                    self.editor_epoch += 1;
                self.status = status.into();
                if let Some(id) = previous.filter(|id| self.binder.get(*id).is_some()) {
                    self.select_node(id);
                } else if let Some(id) = self.opening_scene() {
                    self.select_node(id);
                }
                self.reindex_manuscript();
                self.begin_writing_session();
                self.load_speller();
                self.load_beats();
                self.note_open_problems();
            }
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
                self.status = format!("Backup restored, but reload failed: {err}");
            }
        }
    }

    pub fn undo(&mut self) {
        let structure = self.undo_is_structure_step();
        if let Some(session) = &mut self.session {
            if session.undo() {
                self.editor_text = session.plain_text();
                self.editor_epoch = self.editor_epoch.saturating_add(1);
                self.note_edit();
                self.note_session_for_search();
                if structure {
                    self.undo_structure_step();
                }
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(session) = &mut self.session {
            if session.redo() {
                self.editor_text = session.plain_text();
                self.editor_epoch = self.editor_epoch.saturating_add(1);
                self.note_edit();
                self.note_session_for_search();
            }
        }
    }

    pub fn add_node(&mut self, kind: NodeKind) {
        self.flush_editor();
        let parent_id = self.infer_parent(kind);
        self.create_node(kind, parent_id);
    }

    /// Add a node from the mind map: a child of `from` with Tab (a chapter
    /// in a book or part, a scene in a chapter or folder), or the next one
    /// beside it with Enter, of its kind. It is chosen and its name opened
    /// to type; the status line offers to take it away again.
    pub fn add_from_map(&mut self, from: Id, child: bool) -> Option<Id> {
        let node = self.binder.get(from)?.clone();
        let (kind, parent, after) = if child {
            let kind = match node.kind {
                NodeKind::DraftRoot => NodeKind::Book,
                NodeKind::Book | NodeKind::Part => NodeKind::Chapter,
                // Under Notes, a folder or the root holds notes.
                NodeKind::NotesRoot => NodeKind::Note,
                NodeKind::Folder if self.binder.is_in_notes(from) => NodeKind::Note,
                NodeKind::Chapter | NodeKind::Folder => NodeKind::Scene,
                _ => return None,
            };
            (kind, Some(from), self.binder.children(Some(from)).last().map(|last| last.id))
        } else {
            if node.kind.is_root() {
                return None;
            }
            (node.kind, node.parent_id, Some(from))
        };
        self.flush_editor();
        let id = self.create_node(kind, parent)?;
        if let Some(parent) = parent {
            if self.binder.put_after(id, parent, after).is_ok() {
                self.persist_binder();
            }
        }
        self.folded_map.retain(|folded| Some(*folded) != parent);
        let title = self.binder.get(id).map(|node| node.title.clone()).unwrap_or_default();
        self.status = format!("Added “{title}”. Type its name, then Enter.");
        self.offer_undo(UndoOffer::Added { id });
        self.renaming = Some(id);
        Some(id)
    }

    /// Make a binder item at the end of `parent`, with its document, and
    /// choose it.
    fn create_node(&mut self, kind: NodeKind, parent_id: Option<Id>) -> Option<Id> {
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
            NodeKind::Part => "New part".into(),
            NodeKind::Book => format!("Book {}", self.binder.manuscript_books().len() + 1),
            other => other.icon_label().to_string(),
        };
        let mut node = BinderNode::new(kind, title, parent_id, sort);
        // Under Notes, a new item is compiled as footnotes when its folder is.
        node.footnotes = parent_id.and_then(|parent| self.binder.get(parent)).is_some_and(|parent| parent.footnotes);
        let new_id = node.id;
        if let Some(store) = &self.store {
            if let Err(err) = store.borrow().upsert_node(&node) {
                self.status = format!("Could not add item: {err}");
                return None;
            }
            if let Some(doc_id) = node.document_id {
                let document = writing_studio_document::Document::new(doc_id);
                if let Err(err) = store.borrow().save_document(&document) {
                    self.status = format!("Could not create document: {err}");
                    return None;
                }
            }
        }
        self.binder.nodes.push(node);
        self.select_node(new_id);
        self.status = "Added to binder.".into();
        Some(new_id)
    }

    /// Write the whole project as a folder of Markdown files inside `parent`,
    /// for reading it or for another writing program.
    pub fn export_markdown_folder(&mut self, parent: PathBuf) {
        self.save_all();
        let (Some(store), Some(meta)) = (&self.store, &self.meta) else {
            return;
        };
        let target = parent.join(format!("{} (Markdown)", sanitize(&meta.title)));
        let exported = store.borrow().export_markdown(&target);
        self.status = match exported {
            Ok(count) => format!("Exported {count} files as Markdown to {}.", target.display()),
            Err(err) => format!("Could not export as Markdown: {err}"),
        };
    }

    /// Bring Markdown files in as new scenes, in the order given, where a new
    /// scene would go. A file that cannot be read is named in the status
    /// line, and the others are still brought in.
    pub fn import_markdown_files(&mut self, paths: Vec<PathBuf>) {
        self.flush_editor();
        let Some(store) = self.store.clone() else {
            return;
        };
        let parent = self.infer_parent(NodeKind::Scene);
        let (mut last, mut failed, mut tagged) = (None, Vec::new(), false);
        for path in &paths {
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let sort = self.binder.next_sort_index(parent);
            let imported = std::fs::read_to_string(path)
                .map_err(|err| err.to_string())
                .and_then(|text| {
                    store
                        .borrow()
                        .import_markdown_scene(&text, &name, parent, sort)
                        .map_err(|err| err.to_string())
                });
            match imported {
                Ok((mut node, beats, document)) => {
                    let _ = self.manuscript_index.borrow_mut().upsert(node.id, &document.to_plain_text());
                    if !beats.is_empty() {
                        self.beats.insert(node.id, beats);
                    }
                    let id = node.id;
                    last = Some(id);
                    // Its `tags:` join the project's keywords, a new one
                    // with the next color, one in use as it is spelled.
                    let keywords = std::mem::take(&mut node.keywords);
                    self.binder.nodes.push(node);
                    for word in &keywords {
                        tagged |= self.binder.add_keyword(id, word);
                    }
                }
                Err(err) => failed.push(format!("{name}: {err}")),
            }
        }
        // A label in a file's front matter comes in as a keyword too.
        if self.binder.fold_labels_into_keywords() || tagged {
            self.persist_binder();
        }
        if let Some(id) = last {
            self.select_node(id);
        }
        let count = paths.len() - failed.len();
        self.status = match failed.as_slice() {
            [] => format!("Imported {count} scenes from Markdown."),
            _ => format!("Imported {count} scenes. Not imported: {}", failed.join("; ")),
        };
    }

    pub fn add_plotline(&mut self) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        let id = grid.add_line();
        self.store_plot(grid, true);
        self.selected_plotline = Some(id);
    }

    pub fn set_plotline_title(&mut self, id: Id, title: String) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.set_line_title(id, title) {
            self.store_plot(grid, false);
        }
    }

    pub fn set_plotline_color(&mut self, id: Id, color: String) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.set_line_color(id, color) {
            self.store_plot(grid, true);
        }
    }

    pub fn remove_plotline(&mut self, id: Id) {
        if self.confirm_plotline != Some(id) {
            self.confirm_plotline = Some(id);
            self.status = "Click delete again to remove this plotline and its plot points.".into();
            return;
        }
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.remove_line(id) {
            self.store_plot(grid, true);
        }
    }

    pub fn add_plot_point(&mut self, line_id: Id) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.add_point(line_id).is_some() {
            self.store_plot(grid, true);
        }
    }

    pub fn set_plot_point_title(&mut self, id: Id, title: String) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.set_point_title(id, title) {
            self.store_plot(grid, false);
        }
    }

    pub fn set_plot_point_summary(&mut self, id: Id, summary: String) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.set_point_summary(id, summary) {
            self.store_plot(grid, false);
        }
    }

    pub fn set_plot_point_scene(&mut self, id: Id, scene_id: Option<Id>) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.set_point_scene(id, scene_id) {
            self.store_plot(grid, true);
        }
    }

    pub fn shift_plot_point(&mut self, id: Id, earlier: bool) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.shift_point(id, earlier) {
            self.store_plot(grid, true);
        }
    }

    pub fn remove_plot_point(&mut self, id: Id) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        if grid.remove_point(id) {
            self.store_plot(grid, true);
        }
    }

    pub fn open_linked_scene(&mut self, scene_id: Id) {
        if self.binder.get(scene_id).is_none() {
            self.status = "That is no longer in the binder.".into();
            return;
        }
        self.select_node(scene_id);
        self.mode = WorkspaceMode::Writing;
    }

    fn store_plot(&mut self, grid: PlotGrid, immediate: bool) {
        if !immediate {
            self.plot = grid;
            self.plot_dirty = true;
            self.note_edit();
            return;
        }
        let Some(store) = &self.store else {
            self.plot = grid;
            self.plot_dirty = false;
            return;
        };
        match store.borrow().persist_plot(&grid) {
            Ok(()) => {
                self.plot = grid;
                self.plot_dirty = false;
                self.status = "Plot saved locally.".into();
            }
            Err(err) => self.status = format!("Could not save plot: {err}"),
        }
    }

    fn persist_plot_now(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_plot(&self.plot) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save plot: {err}");
                false
            }
        }
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

    /// The binder row under the pointer, and where on it the drop would go.
    pub fn set_binder_drop(&mut self, id: Id, place: DropPlace) {
        if self.dragging.is_some_and(|dragged| dragged != id)
            && (self.drop_target != Some(id) || self.drop_place != place)
        {
            self.drop_target = Some(id);
            self.drop_place = place;
        }
    }

    /// Whether the row under the pointer can take the dragged item in its
    /// middle, which decides what the middle of the row means.
    pub fn drag_fits_inside(&self, id: Id) -> bool {
        self.dragging
            .and_then(|dragged| self.binder.get(dragged))
            .is_some_and(|node| self.binder.accepts(id, node.kind))
    }

    /// Drop the dragged item on a binder row, at the place the pointer chose.
    pub fn drop_in_binder(&mut self, target: Id) {
        let Some(dragged) = self.dragging.take() else {
            self.drop_target = None;
            return;
        };
        let place = self.drop_place;
        self.drop_target = None;
        self.flush_editor();
        let parent = self.binder.get(dragged).and_then(|node| node.parent_id);
        let after = self.binder.previous_sibling(dragged);
        match self.binder.drop_at(dragged, target, place) {
            Ok(()) => {
                if self.persist_binder() {
                    let title = self.binder.get(dragged).map(|node| node.title.clone()).unwrap_or_default();
                    self.status = format!("Moved “{title}”.");
                    if let Some(parent) = parent {
                        self.offer_undo(UndoOffer::PutBack { id: dragged, parent, after, reopen: false });
                    }
                    if self.selected == Some(dragged) {
                        self.select_node(dragged);
                    }
                }
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging = None;
        self.drop_target = None;
    }

    pub fn trash_selected(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.trash_item(id);
    }

    /// Move any binder item to the trash, with what it holds. The selection
    /// moves to the trash only when it went there too, so trashing another
    /// item from the list leaves the open scene open.
    pub fn trash_item(&mut self, id: Id) {
        self.flush_editor();
        let title = self.binder.get(id).map(|node| node.title.clone()).unwrap_or_default();
        let parent = self.binder.get(id).and_then(|node| node.parent_id);
        let after = self.binder.previous_sibling(id);
        match self.binder.trash(id) {
            Ok(()) => {
                if self.persist_binder() {
                    let selected_gone = self.selected.is_some_and(|selected| self.binder.is_in_trash(selected));
                    if selected_gone {
                        if let Some(trash) = self.binder.trash_root() {
                            let trash_id = trash.id;
                            self.select_node(trash_id);
                        }
                    }
                    self.status = format!("Moved “{title}” to the trash. Restore it from Trash in the library.");
                    if let Some(parent) = parent {
                        self.offer_undo(UndoOffer::PutBack { id, parent, after, reopen: selected_gone });
                    }
                }
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    /// Show an Undo button beside the message just set in the status line.
    pub(crate) fn offer_undo(&mut self, offer: UndoOffer) {
        self.undo_offer = Some((offer, self.status.clone()));
    }

    /// Whether the status line shows an Undo button: only beside the message
    /// it came with, and for a merge or split only while Ctrl+Z would undo it.
    pub fn undo_offered(&self) -> bool {
        match &self.undo_offer {
            Some((UndoOffer::Structure, status)) => *status == self.status && self.undo_is_structure_step(),
            Some((UndoOffer::PutBack { .. } | UndoOffer::Added { .. }, status)) => *status == self.status,
            Some((UndoOffer::Keywords, status)) => *status == self.status && self.keyword_undo.is_some(),
            None => false,
        }
    }

    /// Undo what the status line's Undo button offers.
    pub fn take_undo_offer(&mut self) {
        if !self.undo_offered() {
            return;
        }
        let Some((offer, _)) = self.undo_offer.take() else {
            return;
        };
        match offer {
            UndoOffer::Structure => self.undo(),
            UndoOffer::Added { id } => {
                let title = self.binder.get(id).map(|node| node.title.clone()).unwrap_or_default();
                if self.binder.trash(id).is_ok() && self.persist_binder() {
                    if self.selected == Some(id) {
                        self.selected = None;
                        self.session = None;
                    }
                    self.status = format!("Took “{title}” away again. It is in the trash.");
                }
            }
            UndoOffer::Keywords => {
                let Some(snapshot) = self.keyword_undo.take() else {
                    return;
                };
                for node in &mut self.binder.nodes {
                    node.keywords = snapshot
                        .items
                        .iter()
                        .find(|(id, _)| *id == node.id)
                        .map(|(_, words)| words.clone())
                        .unwrap_or_default();
                }
                self.binder.keywords = snapshot.keywords;
                self.binder.filters = snapshot.filters;
                self.keyword_filter = snapshot.chosen;
                if self.persist_binder() {
                    self.status = "Put the keywords back.".into();
                }
            }
            UndoOffer::PutBack { id, parent, after, reopen } => {
                let title = self.binder.get(id).map(|node| node.title.clone()).unwrap_or_default();
                match self.binder.put_after(id, parent, after) {
                    Ok(()) => {
                        if self.persist_binder() {
                            if reopen {
                                self.select_node(id);
                            }
                            self.status = format!("Put “{title}” back.");
                        }
                    }
                    Err(err) => self.status = err.to_string(),
                }
            }
        }
    }

    /// Open the Go to box, or close it.
    pub fn toggle_quick_open(&mut self) {
        self.open_menu = None;
        self.item_menu = None;
        self.quick_open = !self.quick_open;
    }

    /// The manuscript's books, parts, chapters, folders, and scenes whose
    /// name, or the names of what they are in, have every word typed. Those
    /// whose own name starts with what was typed come first, then those whose
    /// name has every word, then the rest, each in reading order.
    pub fn quick_matches(&self, query: &str) -> Vec<QuickMatch> {
        let Some(draft) = self.binder.draft_root() else {
            return Vec::new();
        };
        let mut items = Vec::new();
        self.collect_quick(draft.id, "", &mut items);
        let query = query.trim().to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut scored: Vec<(u8, QuickMatch)> = items
            .into_iter()
            .filter_map(|item| {
                let title = item.title.to_lowercase();
                let both = format!("{title} {}", item.place.to_lowercase());
                if !words.iter().all(|word| both.contains(word)) {
                    return None;
                }
                let score = if title.starts_with(&query) {
                    0
                } else if words.iter().all(|word| title.contains(word)) {
                    1
                } else {
                    2
                };
                Some((score, item))
            })
            .collect();
        scored.sort_by_key(|(score, _)| *score);
        scored.into_iter().map(|(_, item)| item).collect()
    }

    fn collect_quick(&self, parent: Id, place: &str, items: &mut Vec<QuickMatch>) {
        for child in self.binder.children(Some(parent)) {
            if !matches!(
                child.kind,
                NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Folder | NodeKind::Scene
            ) {
                continue;
            }
            let title = if child.title.trim().is_empty() { "Untitled".to_string() } else { child.title.clone() };
            items.push(QuickMatch { id: child.id, kind: child.kind, title: title.clone(), place: place.to_string() });
            let inner = if place.is_empty() { title } else { format!("{place} › {title}") };
            self.collect_quick(child.id, &inner, items);
        }
    }

    /// Open what the Go to box found: a scene on the page, anything else in
    /// the scene list.
    pub fn go_to(&mut self, id: Id) {
        self.quick_open = false;
        match self.binder.get(id).map(|node| node.kind) {
            Some(NodeKind::Scene) => {
                self.select_node(id);
                self.mode = WorkspaceMode::Writing;
            }
            Some(_) => self.choose_library(id),
            None => {}
        }
    }

    /// Open the menu of a binder item where it was right-clicked. The roots
    /// have none.
    pub fn open_item_menu(&mut self, id: Id, x: f64, y: f64) {
        if self.binder.get(id).is_some_and(|node| !node.kind.is_root()) {
            self.open_menu = None;
            self.item_menu = Some(ItemMenu { id, x, y });
        }
    }

    /// Edit the name of a library row in place. The roots keep theirs.
    pub fn start_rename(&mut self, id: Id) {
        self.item_menu = None;
        if self.binder.get(id).is_some_and(|node| !node.kind.is_root()) {
            self.renaming = Some(id);
        }
    }

    /// Open a scene and join the next one onto it, from the scene's menu.
    pub fn merge_item_with_next(&mut self, id: Id) {
        self.item_menu = None;
        if self.selected != Some(id) {
            self.select_node(id);
        }
        self.mode = WorkspaceMode::Writing;
        self.merge_with_next();
    }

    /// Give a scene a status from its menu.
    pub fn set_item_status(&mut self, id: Id, status: String) {
        self.item_menu = None;
        self.update_node_status(id, status);
        self.save_all();
    }

    /// Take an item out of the trash from its menu.
    pub fn restore_item(&mut self, id: Id) {
        self.item_menu = None;
        self.select_node(id);
        self.restore_selected();
    }

    /// Rename a binder item from the library. An empty name is refused, so
    /// a row never goes blank.
    pub fn rename_item(&mut self, id: Id, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.status = "A name cannot be empty.".into();
            return;
        }
        let Some(old) = self.binder.get(id).filter(|node| node.title != title).map(|node| node.title.clone()) else {
            return;
        };
        let is_note = self.binder.get(id).is_some_and(|node| node.kind == NodeKind::Note);
        let mentioning = if is_note { self.mentions_of(id) } else { Vec::new() };
        self.update_node_title(id, title.to_string());
        self.save_all();
        if !mentioning.is_empty() {
            let changed = self.rename_links_in(&mentioning, &old, title);
            self.status = match changed {
                1 => format!("Renamed, and the link to it in 1 place."),
                many => format!("Renamed, and the links to it in {many} places."),
            };
        }
    }

    /// Point every `[[old]]` in these items' text at `new`. The open one
    /// changes on the page; the others are saved. Returns how many changed.
    fn rename_links_in(&mut self, items: &[Id], old: &str, new: &str) -> usize {
        let Some(store) = self.store.clone() else {
            return 0;
        };
        let open = self.session.as_ref().map(|session| session.document_id());
        let mut changed = 0;
        for item in items {
            let Some(document_id) = self.binder.get(*item).and_then(|node| node.document_id) else {
                continue;
            };
            if Some(document_id) == open {
                let page = self.editor_markdown();
                if let Some(renamed) = writing_studio_format::rename_wikilinks(&page, old, new) {
                    self.edit_text("Links renamed.", |document| {
                        document.replace_blocks(writing_studio_format::markdown_to_blocks(&renamed));
                        true
                    });
                    changed += 1;
                }
                continue;
            }
            let Ok(mut document) = store.borrow().load_document(document_id) else {
                continue;
            };
            let text = writing_studio_format::blocks_to_markdown(&document.blocks);
            let Some(renamed) = writing_studio_format::rename_wikilinks(&text, old, new) else {
                continue;
            };
            document.replace_blocks(writing_studio_format::markdown_to_blocks(&renamed));
            if store.borrow().save_document(&document).is_ok() {
                self.index_links(document_id, &document.to_plain_text());
                changed += 1;
            }
        }
        self.flush_editor();
        if let Some(session) = &self.session {
            let text = session.plain_text();
            self.index_links(session.document_id(), &text);
        }
        changed
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
        let deleted_books: Vec<Id> = self
            .binder
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Book && self.binder.is_in_trash(node.id))
            .map(|node| node.id)
            .collect();
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
                let _ = self.manuscript_index.borrow_mut().remove(*id);
            }
            for id in &nodes {
                if let Err(err) = store.delete_node(*id) {
                    self.status = format!("Could not delete binder item: {err}");
                    return;
                }
            }
        }
        let beats_before = self.beats.len();
        self.beats.retain(|id, _| !nodes.contains(id));
        if self.beats.len() != beats_before && !self.persist_beats_now() {
            self.beats_dirty = true;
        }
        // A note's links to what is gone go with it.
        for node in self.binder.nodes.iter_mut().filter(|node| node.links.iter().any(|id| nodes.contains(id))) {
            node.links.retain(|id| !nodes.contains(id));
        }
        if !self.persist_binder() {
            return;
        }
        if self.plot.detach_scenes(&nodes) && !self.persist_plot_now() {
            self.plot_dirty = true;
            self.note_edit();
            return;
        }
        if self.mindmap.forget(&nodes) {
            self.persist_mindmap();
        }
        if !self.forget_deleted_books(&deleted_books) {
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

    fn forget_deleted_books(&mut self, book_ids: &[Id]) -> bool {
        if book_ids.is_empty() {
            return true;
        }
        let next = self.binder.manuscript_books().first().map(|book| book.id);
        let compile_changed = {
            let Some(meta) = &mut self.meta else {
                return true;
            };
            let mut changed = false;
            let mut compile_changed = false;
            for id in book_ids {
                let was_compile = meta.settings.compile_book_id == Some(*id);
                if meta.settings.drop_book(*id, next) {
                    changed = true;
                    compile_changed |= was_compile;
                }
            }
            if !changed {
                return true;
            }
            meta.touch();
            compile_changed
        };
        if compile_changed {
            if let Some(book) = next {
                self.track_compile_book(book);
            }
        }
        let Some(meta) = self.meta.clone() else {
            return true;
        };
        if let Some(store) = &self.store {
            if let Err(err) = store.borrow().persist_meta(&meta) {
                self.status = format!("Could not update the book target: {err}");
                return false;
            }
        }
        true
    }

    /// Where a new item goes: next to the selection while it is inside the
    /// container chosen in the library, or else in that container, so a
    /// scene added with a chapter chosen lands in that chapter and shows in
    /// its list.
    fn infer_parent(&self, kind: NodeKind) -> Option<Id> {
        let chosen = self
            .library_selection
            .filter(|_| self.mode.shows_binder())
            .and_then(|_| self.shown_container())
            .filter(|id| self.binder.get(*id).is_some_and(|node| node.kind != NodeKind::TrashRoot));
        let anchor = match (chosen, self.selected) {
            (Some(container), Some(selected)) if !self.is_inside(selected, container) => Some(container),
            (Some(container), None) => Some(container),
            _ => self.selected,
        };
        self.binder.parent_for_new_item(kind, anchor, self.series_scope())
    }

    /// Whether `id` is `container` or somewhere below it.
    fn is_inside(&self, id: Id, container: Id) -> bool {
        let mut current = Some(id);
        let mut guard = 0;
        while let Some(at) = current {
            if at == container {
                return true;
            }
            current = self.binder.get(at).and_then(|node| node.parent_id);
            guard += 1;
            if guard > 10_000 {
                return false;
            }
        }
        false
    }

    pub fn update_selected_title(&mut self, title: String) {
        if let Some(id) = self.selected {
            self.update_node_title(id, title);
        }
    }

    pub fn update_selected_synopsis(&mut self, synopsis: String) {
        if let Some(id) = self.selected {
            self.update_node_synopsis(id, synopsis);
        }
    }

    pub fn update_selected_status(&mut self, status: String) {
        if let Some(id) = self.selected {
            self.update_node_status(id, status);
        }
    }

    pub fn update_node_title(&mut self, id: Id, title: String) {
        self.edit_node(id, |node| node.title = title);
    }

    pub fn update_node_synopsis(&mut self, id: Id, synopsis: String) {
        self.edit_node(id, |node| node.synopsis = synopsis);
    }

    /// The writer's notes on an item: kept in its file, never compiled.
    pub fn update_node_notes(&mut self, id: Id, notes: String) {
        self.edit_node(id, |node| node.notes = notes);
    }

    /// Whether a note, or a folder of notes and everything in it, is
    /// compiled as footnotes. Saved at once.
    pub fn set_footnotes(&mut self, id: Id, on: bool) {
        if !self.binder.is_in_notes(id) {
            return;
        }
        let mut ids = vec![id];
        ids.extend(self.items_inside(id));
        for item in ids {
            self.edit_node(item, |node| node.footnotes = on);
        }
        self.save_all();
    }

    /// Whether Context is beside the page: never for a note or a folder of
    /// notes, which are context themselves.
    pub fn context_shown(&self) -> bool {
        self.mode.shows_inspector() && !self.selected.is_some_and(|id| self.binder.is_in_notes(id))
    }

    pub fn update_node_status(&mut self, id: Id, status: String) {
        self.edit_node(id, |node| node.status = status);
    }

    pub fn update_selected_label(&mut self, label: String) {
        let label: String = label.chars().take(80).collect();
        if let Some(id) = self.selected {
            self.edit_node(id, |node| node.label = label);
        }
    }

    pub fn update_selected_color(&mut self, color: String) {
        let color = normalize_binder_color(&color);
        if let Some(id) = self.selected {
            self.edit_node(id, |node| node.color = color);
        }
    }

    /// An item's color, from its right-click menu, saved at once.
    pub fn set_item_color(&mut self, id: Id, color: &str) {
        let color = normalize_binder_color(color);
        self.edit_node(id, |node| node.color = color);
        self.save_all();
    }

    fn edit_node(&mut self, id: Id, apply: impl FnOnce(&mut BinderNode)) {
        let Some(node) = self.binder.get_mut(id) else {
            return;
        };
        apply(node);
        node.updated_at = writing_studio_core::Timestamp::now();
        self.binder_dirty = true;
        self.note_edit();
    }
}

pub struct ExportFile {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub filter_name: &'static str,
    pub extension: &'static str,
    pub kind_label: &'static str,
    /// What was compiled, in a few words: "12 chapters, 84210 words".
    pub summary: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_document::ParagraphChange;

    /// The middle of the window has no view switch: the page opens from a
    /// scene or a note, and every other mode from one row under Views, so
    /// none is unreachable or listed twice.
    #[test]
    fn every_mode_but_the_page_has_one_library_row() {
        let mut rows: Vec<WorkspaceMode> = WorkspaceMode::LIBRARY_VIEWS.to_vec();
        rows.push(WorkspaceMode::Writing);
        for mode in WorkspaceMode::ALL {
            assert_eq!(rows.iter().filter(|row| **row == mode).count(), 1, "{mode:?}");
        }
        assert_eq!(rows.len(), WorkspaceMode::ALL.len());
    }

    /// A keyword chosen in the library's list narrows the scene list to
    /// what carries it or one nested under it; choosing it again, or
    /// Escape's `clear_keyword_filter`, lets it go.
    #[test]
    fn a_keyword_from_the_library_narrows_the_scene_list_and_folds() {
        let (mut state, dir) = open_state("keyword-list");
        let tagged = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let other = state.selected.unwrap();
        state.add_keyword(tagged, "plot/subplot");
        state.add_keyword(other, "flashback");
        let shown = |state: &StudioState| -> Vec<Id> {
            state.binder_rows().into_iter().filter(|row| row.kind == NodeKind::Scene).map(|row| row.id).collect()
        };
        assert_eq!(shown(&state), [tagged, other]);

        state.mode = WorkspaceMode::Plot;
        state.choose_keyword("Plot");
        assert_eq!(state.mode, WorkspaceMode::Writing, "the list is beside the page");
        assert_eq!(shown(&state), [tagged], "plot brings plot/subplot");
        state.choose_keyword("plot");
        assert_eq!(shown(&state), [tagged, other], "the same keyword again lets it go");
        state.choose_keyword("flashback");
        assert!(state.clear_keyword_filter() && !state.clear_keyword_filter());

        let names = |state: &StudioState| -> Vec<String> { state.keyword_list().into_iter().map(|row| row.name).collect() };
        assert_eq!(names(&state), ["flashback", "plot", "plot/subplot"]);
        state.toggle_keyword_fold("PLOT");
        assert_eq!(names(&state), ["flashback", "plot"], "a folded keyword hides what is under it");
        state.toggle_keyword_fold("plot");
        state.add_keyword(other, "plot/twist");
        state.keyword_sort = KeywordSort::Count;
        assert_eq!(names(&state), ["plot", "plot/subplot", "plot/twist", "flashback"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A rename is saved and offers Undo; one that would merge two
    /// keywords waits for a second Enter, and Escape lets it go.
    #[test]
    fn a_keyword_is_renamed_merged_and_put_back() {
        let (mut state, dir) = open_state("keyword-rename");
        let tagged = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let other = state.selected.unwrap();
        state.add_keyword(tagged, "plot/subplot");
        state.add_keyword(other, "story");
        state.choose_keyword("plot");
        state.renaming_keyword = Some("plot".into());

        state.rename_keyword("plot", "thread");
        assert_eq!(state.binder.get(tagged).unwrap().keywords, ["thread/subplot"]);
        assert!(state.status.contains("on 1 item"), "{}", state.status);
        assert_eq!(state.renaming_keyword, None);
        assert_eq!(state.keyword_filter.as_deref(), Some("thread"), "the chosen keyword follows");
        let binder = state.project_root().unwrap().join("binder.json");
        let saved = std::fs::read_to_string(&binder).unwrap();
        assert!(saved.contains("thread/subplot") && !saved.contains("plot/subplot"));

        assert!(state.undo_offered());
        state.take_undo_offer();
        assert_eq!(state.binder.get(tagged).unwrap().keywords, ["plot/subplot"]);
        assert_eq!(state.keyword_filter.as_deref(), Some("plot"));
        assert!(std::fs::read_to_string(&binder).unwrap().contains("plot/subplot"));

        state.rename_keyword("plot", "Story");
        assert!(state.status.contains("Enter again"), "{}", state.status);
        assert_eq!(state.binder.get(tagged).unwrap().keywords, ["plot/subplot"], "nothing yet");
        assert!(state.cancel_confirmation(), "Escape lets the merge go");
        state.rename_keyword("plot", "Story");
        state.rename_keyword("plot", "Story");
        assert_eq!(state.binder.get(tagged).unwrap().keywords, ["story/subplot"], "spelled as the one in use");
        assert!(state.status.starts_with("Merged"), "{}", state.status);

        state.rename_keyword("story", "a//b");
        assert!(state.status.contains("not a keyword"));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A note linked to a chapter reaches every scene in it: a saved filter
    /// "with" the note lists them, and emptying the trash of the chapter
    /// takes the link away.
    #[test]
    fn a_note_linked_to_a_chapter_reaches_its_scenes() {
        let (mut state, dir) = open_state("note-reach");
        let first = state.selected.unwrap();
        let chapter = state.binder.get(first).unwrap().parent_id.unwrap();
        state.add_node(NodeKind::Scene);
        let second = state.selected.unwrap();
        state.add_node(NodeKind::Chapter);
        state.add_node(NodeKind::Scene);
        let elsewhere = state.selected.unwrap();
        let mara = add_note(&mut state, "Mara", "");
        state.link_note_titled(chapter, "mara");
        assert!(state.note_reaches(mara, first) && state.note_reaches(mara, second));
        assert!(!state.note_reaches(mara, elsewhere), "another chapter's scene is not reached");

        let shown = |state: &StudioState| -> Vec<Id> {
            state.binder_rows().into_iter().filter(|row| row.kind == NodeKind::Scene).map(|row| row.id).collect()
        };
        state.choose_library(state.binder.book_of(first).unwrap());
        state.add_filter(SavedFilter {
            id: Id::new(),
            name: String::new(),
            keywords: Vec::new(),
            any: false,
            without: Vec::new(),
            entries: vec![mara],
            status: String::new(),
            text: String::new(),
        });
        assert_eq!(state.binder.filters[0].name, "with Mara");
        assert_eq!(shown(&state), [first, second]);
        state.remove_filter(state.binder.filters[0].id);

        state.select_node(chapter);
        state.trash_selected();
        state.empty_trash();
        state.empty_trash();
        assert!(state.binder.get(mara).unwrap().links.is_empty(), "a link to what is gone goes");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Entries the open scene names are offered as links once it is saved,
    /// never linked by themselves; one click links, another hides it.
    #[test]
    fn names_in_a_saved_scene_are_offered_as_links() {
        let (mut state, dir) = open_state("named-here");
        let scene = state.selected.unwrap();
        let mara = add_note(&mut state, "Mara Vance", "");
        add_note(&mut state, "Rose", "");
        let port = add_note(&mut state, "Port Ely", "");
        add_note(&mut state, "The Old War", "");
        state.select_node(scene);

        state.apply_editor_text("Vance ran down to port Ely as the sun rose.".into());
        assert!(state.named_here.is_empty(), "nothing is looked for before the save");
        state.flush_editor();
        assert_eq!(state.named_here, [mara, port], "Rose is not in “rose”; the war is not named");
        assert!(state.binder.get(mara).unwrap().links.is_empty(), "nothing is linked by itself");

        state.link_named(mara);
        assert_eq!(state.binder.get(mara).unwrap().links, [scene]);
        assert_eq!(state.named_here, [port]);
        state.hide_named(port);
        assert!(state.named_here.is_empty());
        state.set_note_link(mara, scene, false);
        state.flush_editor();
        assert_eq!(state.named_here, [mara], "unlinked, it is offered again");

        // Linked through its chapter, it is not offered.
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        state.set_note_link(mara, chapter, true);
        state.flush_editor();
        assert!(state.named_here.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The open scene's context, from the context crate, holds what its
    /// chapter's links bring; "Copy context" writes it as Markdown.
    #[test]
    fn the_scene_context_holds_what_reaches_the_scene() {
        let (mut state, dir) = open_state("scene-context");
        let scene = state.selected.unwrap();
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        let mara = add_note(&mut state, "Mara", "Always late.");
        let port = add_note(&mut state, "Port Ely", "A harbor town.");
        state.set_note_link(mara, chapter, true);
        state.select_node(scene);
        state.apply_editor_markdown("They sailed from [[Port Ely]].");
        state.flush_editor();
        let context = state.scene_context(scene).unwrap();
        let entries: Vec<(&str, Option<&str>)> = context.entries.iter().map(|entry| (entry.name.as_str(), entry.through.as_deref())).collect();
        assert_eq!(entries, [("Mara", Some("Chapter 1")), ("Port Ely", None)]);
        let markdown = context.to_markdown(16_000);
        assert!(markdown.contains("## Research") && markdown.contains("### Mara") && markdown.contains("Always late."), "{markdown}");
        assert!(state.scene_context(chapter).is_none());

        // The chapter has its own: both notes are in its scene.
        let level = state.level_context(chapter).unwrap();
        assert_eq!(level.entries.len(), 2);
        assert!(level.entries.iter().all(|entry| entry.scenes == 1));
        assert!(level.outline.iter().any(|item| item.depth == 0 && Some(item.title.as_str()) == state.binder.get(scene).map(|node| node.title.as_str())));
        assert!(state.level_context(scene).is_none());

        // Context on the scene lists them, and links one by its title.
        let (own, through) = state.notes_reaching(scene);
        assert_eq!(own.iter().map(|(id, _, linked)| (*id, *linked)).collect::<Vec<_>>(), [(port, false)]);
        assert_eq!(through.iter().map(|(id, _, from)| (*id, from.as_str())).collect::<Vec<_>>(), [(mara, "Chapter 1")]);
        state.link_note_titled(scene, "port ely");
        assert_eq!(state.binder.get(port).unwrap().links, [scene]);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The mind map has the only book in its middle, or the whole
    /// manuscript once there are two; it folds, zooms, and fits again, and
    /// a node chosen on it shows in the inspector.
    #[test]
    fn the_mind_map_has_its_book_in_the_middle_and_folds_and_zooms() {
        let (mut state, dir) = open_state("mind-map");
        let scene = state.selected.unwrap();
        let book = state.binder.book_of(scene).unwrap();
        assert_eq!(state.map_root(), Some(book));
        assert!(WorkspaceMode::MindMap.shows_inspector());
        let map = crate::mindmap::layout(&state.binder, book, &state.folded_map);
        assert!(map.get(scene).is_some());

        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        state.toggle_map_fold(chapter);
        assert!(crate::mindmap::layout(&state.binder, book, &state.folded_map).get(scene).is_none());
        state.toggle_map_fold(chapter);
        assert!(state.folded_map.is_empty());

        state.zoom_map(Some(1.25));
        assert_eq!(state.map_zoom, Some(1.25), "from fitted, a zoom starts at natural size");
        state.zoom_map(Some(100.0));
        assert_eq!(state.map_zoom, Some(3.0));
        state.zoom_map(None);
        assert_eq!(state.map_zoom, None);

        state.choose_library(book);
        state.add_node(NodeKind::Book);
        assert_eq!(state.map_root(), state.binder.draft_root().map(|draft| draft.id), "two books: the manuscript");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// From a chosen node, Tab adds a child of the next kind down and Enter
    /// one of its own kind right after it, named as it comes; a move by
    /// dragging, and an addition, are each undone from the status line.
    #[test]
    fn the_mind_map_adds_children_and_siblings_and_moves_with_undo() {
        let (mut state, dir) = open_state("map-edit");
        let first = state.selected.unwrap();
        let chapter = state.binder.get(first).unwrap().parent_id.unwrap();
        let book = state.binder.get(chapter).unwrap().parent_id.unwrap();

        let added = state.add_from_map(chapter, false).unwrap();
        let node = state.binder.get(added).unwrap().clone();
        assert_eq!((node.kind, node.parent_id), (NodeKind::Chapter, Some(book)));
        assert_eq!(state.binder.previous_sibling(added), Some(chapter), "right after the chosen one");
        assert_eq!((state.selected, state.renaming), (Some(added), Some(added)), "chosen, with its name open");
        let scene = state.add_from_map(added, true).unwrap();
        assert_eq!(state.binder.get(scene).unwrap().kind, NodeKind::Scene);
        assert_eq!(state.binder.get(scene).unwrap().parent_id, Some(added));
        assert_eq!(state.add_from_map(scene, true), None, "a scene holds no node");
        assert!(state.add_from_map(book, false).is_some(), "a book beside the book");

        // Undo takes the last addition away again, into the trash.
        let other_book = state.selected.unwrap();
        assert!(state.undo_offered());
        state.take_undo_offer();
        assert!(state.binder.is_in_trash(other_book));

        // Dragged into the new chapter, the first scene moves; Undo puts it back.
        state.begin_drag(first);
        state.set_binder_drop(added, DropPlace::Inside);
        state.drop_in_binder(added);
        assert_eq!(state.binder.get(first).unwrap().parent_id, Some(added));
        assert!(state.status.starts_with("Moved"), "{}", state.status);
        state.take_undo_offer();
        assert_eq!(state.binder.get(first).unwrap().parent_id, Some(chapter));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Ideas, lines, and hand-placed nodes on the mind map, saved in
    /// `mindmap.json`: an idea becomes a scene with its lines, and emptying the trash
    /// takes away the lines of what it deleted.
    #[test]
    fn ideas_lines_and_places_on_the_mind_map() {
        let (mut state, dir) = open_state("map-ideas");
        let scene = state.selected.unwrap();
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        let book = state.map_root().unwrap();

        let idea = state.add_idea(Some(chapter));
        state.set_idea_title(idea, "Storm at sea".into());
        state.set_idea_note(idea, "The ship nearly goes down.".into());
        assert_eq!(state.map_chosen(), Some(idea));
        state.start_join(idea);
        state.choose_on_map(scene);
        assert_eq!(state.mindmap.lines.len(), 1, "a line from the idea to the scene");
        let line = state.mindmap.lines[0].id;
        state.set_line_label(line, "foreshadows".into());
        let map_file = state.project_root().unwrap().join("mindmap.json");
        assert!(std::fs::read_to_string(&map_file).unwrap().contains("foreshadows"));
        let map = state.map_layout(book);
        assert_eq!(map.get(idea).unwrap().kind, crate::mindmap::MapKind::Idea);

        // Placed by hand, then laid out again.
        state.place_on_map(idea, 500.0, 600.0);
        assert_eq!((state.map_layout(book).get(idea).unwrap().x, state.map_layout(book).get(idea).unwrap().y), (500.0, 600.0));
        state.tidy_map();
        assert!(state.mindmap.positions.is_empty());

        // Dropped on the chapter, the idea becomes a scene there; its line goes with it.
        state.begin_drag(idea);
        state.set_binder_drop(chapter, DropPlace::Inside);
        state.drop_on_map(chapter);
        let made = state.selected.unwrap();
        let node = state.binder.get(made).unwrap().clone();
        assert_eq!((node.title.as_str(), node.synopsis.as_str()), ("Storm at sea", "The ship nearly goes down."));
        assert_eq!(node.parent_id, Some(chapter));
        assert!(state.mindmap.ideas.is_empty());
        assert_eq!((state.mindmap.lines[0].from, state.mindmap.lines[0].to), (made, scene));

        // Emptied from the trash, the scene's line goes.
        state.select_node(made);
        state.trash_selected();
        state.empty_trash();
        state.empty_trash();
        assert!(state.mindmap.lines.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// "Compile as footnotes" belongs to each note: setting it on a folder
    /// sets everything in it, a new note takes its folder's, and Context is
    /// never beside a note or a folder of notes.
    #[test]
    fn footnotes_cascade_from_a_folder_and_context_stays_off_notes() {
        let (mut state, dir) = open_state("footnote-cascade");
        let scene = state.selected.unwrap();
        assert!(state.context_shown(), "beside a scene");
        let notes = state.binder.notes_root().unwrap().id;
        let (footnotes, research) = (state.binder.children(Some(notes))[0].id, state.binder.children(Some(notes))[1].id);
        let cited = state.new_note_from_template(footnotes, "source.md").unwrap();
        assert!(state.binder.get(cited).unwrap().footnotes, "made in Footnotes");
        assert!(!state.context_shown(), "not beside a note");
        state.select_node(research);
        assert!(!state.context_shown(), "nor a folder of notes");
        state.select_node(research);
        state.add_node(NodeKind::Folder);
        let inner = state.selected.unwrap();
        let deep = state.new_note_from_template(inner, "research.md").unwrap();
        assert!(!state.binder.get(deep).unwrap().footnotes);

        state.set_footnotes(research, true);
        assert!([research, inner, deep].iter().all(|id| state.binder.get(*id).unwrap().footnotes), "the folder and all in it");
        state.set_footnotes(deep, false);
        assert!(state.binder.get(inner).unwrap().footnotes && !state.binder.get(deep).unwrap().footnotes, "one note apart");
        state.set_footnotes(scene, true);
        assert!(!state.binder.get(scene).unwrap().footnotes, "only under Notes");
        let dots: Vec<(Id, Option<bool>)> = state.note_rows().into_iter().map(|row| (row.id, row.compiles)).collect();
        assert!(dots.contains(&(inner, Some(true))) && dots.contains(&(deep, Some(false))), "the library's dots");
        assert!(state.binder_rows().iter().all(|row| row.compiles.is_none()), "none in the manuscript");
        state.set_item_color(deep, "blue");
        assert_eq!(state.binder.get(deep).unwrap().color, "blue");
        state.set_item_color(deep, "plaid");
        assert_eq!(state.binder.get(deep).unwrap().color, "", "not a binder color");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The library's Notes section holds the folders and notes under Notes,
    /// and the search narrows it.
    #[test]
    fn the_library_lists_notes_and_a_search_narrows_them() {
        let (mut state, dir) = open_state("note-rows");
        let mara = add_note(&mut state, "Mara", "A pilot's daughter.");
        add_note(&mut state, "Port Ely", "A harbor town.");
        let rows: Vec<(String, u32)> = state.note_rows().into_iter().map(|row| (row.title, row.depth)).collect();
        assert_eq!(rows[0], ("Footnotes".to_string(), 0));
        assert_eq!(rows[1], ("Research".to_string(), 0));
        assert!(rows.contains(&("Mara".to_string(), 1)) && rows.contains(&("Port Ely".to_string(), 1)), "{rows:?}");

        state.search = "harbor".into();
        let found: Vec<String> = state.note_rows().into_iter().map(|row| row.title).collect();
        assert_eq!(found, ["Research", "Port Ely"], "the note that matches and its folder");
        state.search.clear();

        // + Note adds inside the chosen folder of notes.
        let research = state.binder.get(mara).unwrap().parent_id.unwrap();
        state.select_node(research);
        let added = state.add_in_notes(NodeKind::Note).unwrap();
        assert_eq!(state.binder.get(added).unwrap().parent_id, Some(research));
        assert_eq!(state.selected, Some(added));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_notes_map_is_the_notes_and_tab_adds_a_note() {
        let (mut state, dir) = open_state("notes-map");
        state.map_notes = true;
        let (root, map) = state.current_map().unwrap();
        assert_eq!(Some(root), state.binder.notes_root().map(|node| node.id));
        let research = state.binder.children(Some(root))[1].id;
        assert!(map.get(research).is_some(), "its folders are on it");
        let added = state.add_from_map(research, true).unwrap();
        let node = state.binder.get(added).unwrap();
        assert_eq!((node.kind, node.parent_id), (NodeKind::Note, Some(research)));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A note in the Research folder, with this title and text.
    fn add_note(state: &mut StudioState, title: &str, text: &str) -> Id {
        let notes = state.binder.notes_root().unwrap().id;
        let research = state.binder.children(Some(notes))[1].id;
        let note = state.new_note_from_template(research, "research.md").unwrap();
        state.rename_item(note, title);
        state.apply_editor_markdown(text);
        state.flush_editor();
        note
    }

    fn open_state(tag: &str) -> (StudioState, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ws-ui-{tag}-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut state = StudioState::default();
        state.create_project_in(dir.clone());
        assert!(state.session.is_some(), "{}", state.status);
        (state, dir)
    }

    #[test]
    fn snapshot_comparison_reads_unsaved_text_and_other_snapshots() {
        let (mut state, dir) = open_state("compare");
        assert!(state.snapshot_comparison().is_none());

        state.apply_editor_text("She opened the door.".into());
        state.snapshot();
        state.apply_editor_text("She opened the red door.\nNobody was there.".into());
        state.snapshot();
        // Typed after the last snapshot and not saved yet.
        state.apply_editor_text("She opened the red door.".into());

        let snapshots = state.snapshots_for_selected();
        assert_eq!(snapshots.len(), 2);
        let (newest, oldest) = (snapshots[0].id, snapshots[1].id);

        state.compare_snapshot(oldest);
        let against_current = state.snapshot_comparison().unwrap().unwrap();
        assert_eq!(against_current.newer, "Current text");
        assert_eq!(against_current.diff.paragraphs.len(), 1);
        assert_eq!(against_current.diff.paragraphs[0].change, ParagraphChange::Changed);
        assert_eq!(against_current.diff.words_added, 1);

        // Either snapshot can be the one chosen first; the older stays on the left.
        state.set_compare_against(Some(newest));
        let between = state.snapshot_comparison().unwrap().unwrap();
        assert_eq!(between.diff.paragraphs.len(), 2);
        assert_eq!(between.diff.paragraphs[1].change, ParagraphChange::Added);
        state.compare_snapshot(newest);
        state.set_compare_against(Some(oldest));
        assert_eq!(state.snapshot_comparison().unwrap().unwrap().diff, between.diff);

        // Comparing changes nothing on disk.
        assert_eq!(state.snapshots_for_selected().len(), 2);
        assert_eq!(state.editor_text, "She opened the red door.");

        state.compare_snapshot(newest);
        assert!(state.snapshot_comparison().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn review_edits_redraw_the_editor_and_save_with_the_scene() {
        let (mut state, dir) = open_state("review");
        state.apply_editor_markdown("The {==harbor==}{>>which one?<<} was {==quiet==}.\n\n{==Too long.==}{>>cut<<}");
        assert_eq!(state.review_items().len(), 3);
        let epoch = state.editor_epoch;

        state.set_note_kind(0, NoteKind::Question);
        state.resolve_note(0, true);
        state.set_note_text(1, "cut this");
        assert!(state.editor_epoch > epoch);
        let page = state.editor_markdown();
        assert!(page.contains("{>>(resolved) Question: which one?<<}"), "{page}");

        // What the page sends back after the redraw is not a new edit.
        let undo_to = |state: &StudioState| state.session.as_ref().unwrap().undo_target().map(|doc| doc.to_html());
        let before = undo_to(&state);
        state.apply_editor_markdown(&page);
        assert_eq!(undo_to(&state), before);

        state.flush_editor();
        let doc_id = state.session.as_ref().unwrap().document_id();
        let saved = state.store.as_ref().unwrap().borrow().load_document(doc_id).unwrap();
        let items = saved.review_items();
        assert_eq!(
            items[0].kind,
            ReviewKind::Note {
                comment: "which one?".into(),
                kind: NoteKind::Question,
                resolved: true,
            }
        );
        assert!(matches!(&items[2].kind, ReviewKind::Note { comment, .. } if comment == "cut this"));

        state.remove_review(ReviewTarget::Highlight(0));
        state.remove_resolved_notes();
        let left = state.review_items();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].text, "Too long.");
        assert_eq!(state.editor_text, "The harbor was quiet.\nToo long.");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rich_text_pasted_on_the_markdown_page_comes_in_as_markdown() {
        // What a browser on Windows puts on the clipboard: a fragment marked
        // inside a page with a style sheet and a title.
        let clipboard = concat!(
            "<html><head><title>Notes</title><style>p { color: red; }</style></head><body>",
            "<!--StartFragment--><p>She opened the <b>red</b> door.</p>",
            "<p>A <i>gull</i> cried.<o:p></o:p></p><!--EndFragment--></body></html>"
        );
        assert_eq!(pasted_markdown(clipboard), "She opened the **red** door.

A *gull* cried.");
        // One paragraph is a line, to go into the paragraph it is pasted in.
        assert_eq!(pasted_markdown("<span>a <a href=\"https://example.com\">link</a></span>"), "a [link](https://example.com)");
        assert_eq!(pasted_markdown("<style>p {}</style><p> </p>"), "");
        assert_eq!(pasted_markdown("<header>Kept</header>"), "Kept", "a tag that only starts like head is not head");
    }

    #[test]
    fn the_markdown_page_reads_and_writes_the_scene() {
        let (mut state, dir) = open_state("markdown-page");
        state.apply_editor_markdown("");
        assert_eq!(state.editor_markdown(), "", "an empty scene is an empty page");
        state.apply_editor_markdown("She opened the **red** door.\n\nNobody spoke.");
        assert_eq!(state.editor_markdown(), "She opened the **red** door.

Nobody spoke.");
        let ids: Vec<_> = state.session.as_ref().unwrap().document().blocks.iter().map(|block| block.id).collect();
        let epoch = state.editor_epoch;

        // A change from the page: the document follows, keeps its blocks'
        // ids, and the page is not drawn again for it.
        state.apply_editor_markdown("She opened the *red* door.

Nobody spoke. A gull cried.");
        let document = state.session.as_ref().unwrap().document().clone();
        assert_eq!(document.blocks.iter().map(|block| block.id).collect::<Vec<_>>(), ids);
        assert_eq!(state.editor_text, "She opened the red door.
Nobody spoke. A gull cried.");
        assert_eq!(state.editor_epoch, epoch);
        assert!(state.has_unsaved());
        // The same text again is no change and no undo step.
        state.apply_editor_markdown("She opened the *red* door.

Nobody spoke. A gull cried.");
        state.undo();
        assert_eq!(state.editor_markdown(), "She opened the **red** door.

Nobody spoke.");
        assert!(state.editor_epoch > epoch, "an undo redraws the page");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tracked_changes_are_accepted_rejected_and_kept_across_reopen() {
        let (mut state, dir) = open_state("track");
        state.apply_editor_markdown("She opened the door.\n\nNobody spoke.");
        assert!(!state.is_tracking());
        assert!(state.tracked_changes().is_empty());

        state.start_tracking();
        assert!(state.is_tracking());
        assert!(state.tracked_changes().is_empty());

        state.apply_editor_markdown("She opened the red door.\n\nA gull cried.\n\nNobody spoke.\n\nThe end.");
        assert_eq!(state.tracked_changes().len(), 3);

        // Accepting keeps the text and drops the change.
        state.accept_tracked_change(0);
        assert_eq!(state.tracked_changes().len(), 2);
        assert!(state.editor_text.starts_with("She opened the red door."));

        // Rejecting takes the paragraph back out of the text, and undo returns it.
        let epoch = state.editor_epoch;
        state.reject_tracked_change(0);
        assert_eq!(state.editor_text, "She opened the red door.\nNobody spoke.\nThe end.");
        assert!(state.editor_epoch > epoch);
        assert_eq!(state.tracked_changes().len(), 1);
        state.undo();
        assert_eq!(state.tracked_changes().len(), 2);

        // The baseline is in the project, so the changes survive closing it.
        state.flush_editor();
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let mut reopened = StudioState::default();
        reopened.open_project_at(root);
        assert!(reopened.is_tracking());
        assert_eq!(reopened.tracked_changes().len(), 2);

        reopened.reject_all_tracked_changes();
        assert_eq!(reopened.editor_text, "She opened the red door.\nNobody spoke.");
        assert!(reopened.tracked_changes().is_empty());
        reopened.undo();
        reopened.accept_all_tracked_changes();
        assert!(reopened.tracked_changes().is_empty());
        assert!(reopened.editor_text.ends_with("The end."));

        reopened.stop_tracking();
        assert!(!reopened.is_tracking());
        assert!(reopened.editor_text.ends_with("The end."));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn spelling_uses_the_dictionary_project_words_and_note_titles() {
        let (mut state, dir) = open_state("spell");
        assert_eq!(state.spelling_language().as_deref(), Some("en_US"));
        state.apply_editor_markdown("Maravance did not recieve teh letter. Teh end, said Zhul.");
        let words: Vec<String> = state.unknown_words().into_iter().map(|item| item.word).collect();
        assert_eq!(words, ["Maravance", "recieve", "teh", "Teh", "Zhul"]);
        assert!(state.spelling_suggestions("recieve").contains(&"receive".to_string()));

        state.add_project_word("Zhul");
        state.add_project_word("zhul");
        assert_eq!(state.project_words(), ["zhul"]);
        let scene = state.selected.unwrap();
        add_note(&mut state, "Maravance Oduya", "A pilot.");
        state.select_node(scene);
        state.replace_misspelling("recieve", "receive");
        state.replace_misspelling("teh", "the");
        let words: Vec<String> = state.unknown_words().into_iter().map(|item| item.word).collect();
        assert_eq!(words, ["Teh"]);
        assert!(state.editor_text.contains("receive the letter"));
        state.undo();
        assert!(state.editor_text.contains("receive teh letter"));

        // The switch, the language, and the word list are saved in the project.
        state.set_spelling(None);
        assert!(state.unknown_words().is_empty());
        state.save_all();
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let mut reopened = StudioState::default();
        reopened.open_project_at(root);
        assert!(reopened.spelling_language().is_none());
        assert_eq!(reopened.project_words(), ["zhul"]);
        reopened.set_spelling(Some("missing_XX".into()));
        assert_eq!(reopened.spelling_language().as_deref(), Some("en_US"));
        reopened.remove_project_word("zhul");
        assert!(reopened.unknown_words().iter().any(|item| item.word == "Zhul"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn scene_diagnostics_report_beats_and_text_and_save_with_the_project() {
        let (mut state, dir) = open_state("diagnostics");
        let scene = state.selected.unwrap();
        assert!(state.has_scene_diagnostics(scene));
        let manuscript = state
            .binder
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::DraftRoot)
            .map(|node| node.id)
            .unwrap();
        assert!(!state.has_scene_diagnostics(manuscript));

        let report = state.scene_report(scene).unwrap();
        // A new scene has its opening line and no beats yet.
        assert_eq!(
            report.findings,
            [Finding::NoGoal, Finding::NoConflict, Finding::NoOutcome]
        );

        add_note(&mut state, "Mara Vance", "A pilot's daughter.");
        state.select_node(scene);
        state.apply_editor_markdown("Mara ran for the harbor. “Wait,” said the pilot.");
        state.set_scene_beat(scene, Beat::Goal, "Reach the harbor".into());
        state.set_scene_beat(scene, Beat::Outcome, "The boat has gone".into());
        assert!(state.has_unsaved());
        let report = state.scene_report(scene).unwrap();
        assert_eq!(report.findings, [Finding::NoConflict]);
        assert_eq!(report.facts.words, 9);
        assert_eq!(report.facts.characters[0].name, "Mara Vance");
        assert_eq!(report.facts.dialogue_percent, 11);

        state.save_all();
        assert!(!state.has_unsaved());
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let mut reopened = StudioState::default();
        reopened.open_project_at(root);
        assert_eq!(reopened.scene_beats(scene).goal, "Reach the harbor");
        reopened.load_diagnostic_texts();
        // Another scene is on screen; the table still reads this one's text.
        reopened.session = None;
        assert_eq!(reopened.scene_report(scene).unwrap().facts.words, 9);

        // Emptying the trash drops the beats of deleted scenes.
        reopened.select_node(scene);
        reopened.trash_selected();
        reopened.empty_trash();
        reopened.empty_trash();
        assert!(reopened.beats.is_empty());
        let store = reopened.store.as_ref().unwrap().borrow();
        assert!(store.load_scene_beats().unwrap().is_empty());
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The file of the open scene, found by the id in it.
    fn open_scene_file(state: &StudioState) -> PathBuf {
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let id = state.session.as_ref().unwrap().document_id().to_string();
        std::fs::read_dir(root.join("manuscript"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| std::fs::read_to_string(path).unwrap_or_default().contains(&id))
            .unwrap()
    }

    #[test]
    fn escape_cancels_a_delete_waiting_for_its_second_click() {
        let (mut state, dir) = open_state("cancel");
        state.add_plotline();
        let id = state.plot.lines[0].id;
        state.remove_plotline(id);
        assert_eq!(state.confirm_plotline, Some(id));
        assert!(state.cancel_confirmation());
        assert!(!state.cancel_confirmation());
        // The next click asks again instead of deleting.
        state.remove_plotline(id);
        assert_eq!(state.plot.lines.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn choosing_the_open_mode_does_nothing() {
        let mut state = StudioState::default();
        assert!(!state.choose_mode(WorkspaceMode::Writing));
        assert!(state.choose_mode(WorkspaceMode::Plot));
        assert_eq!(state.mode, WorkspaceMode::Plot);
        assert!(!state.choose_mode(WorkspaceMode::Plot));
        // From focus mode, the same mode brings the window back.
        state.focus_mode = true;
        assert!(state.choose_mode(WorkspaceMode::Plot));
        assert!(!state.focus_mode);
    }

    /// The binder lists the scenes in their chapters, under the books and
    /// parts they are in, each row with what it shows; a search keeps the
    /// scenes that match and what they are in.
    #[test]
    fn the_binder_holds_the_scenes_in_their_chapters_and_a_search_narrows_it() {
        let (mut state, dir) = open_state("scene-list");
        let book = state.binder.manuscript_books()[0].id;
        state.choose_library(book);
        state.mode = WorkspaceMode::Plot;
        state.choose_library(book);
        assert_eq!(state.mode, WorkspaceMode::Writing);
        assert_eq!(state.shown_container(), Some(book));
        state.apply_editor_text("Finch had learned that the desert kept nothing.".into());
        state.save_all();
        let scene = state.selected.unwrap();
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        state.update_node_status(scene, "Done".into());
        let rows = state.binder_rows();
        let depth = |id: Id| rows.iter().find(|row| row.id == id).map(|row| row.depth);
        assert_eq!((depth(book), depth(chapter), depth(scene)), (Some(0), Some(1), Some(2)), "nested");
        let chapter_row = rows.iter().find(|row| row.id == chapter).unwrap();
        assert_eq!((chapter_row.scenes, chapter_row.finished), (1, 1), "its ring");

        state.add_node(NodeKind::Scene);
        let other = state.selected.unwrap();
        state.search = "desert".into();
        let shown: Vec<Id> = state.binder_rows().iter().map(|row| row.id).collect();
        assert!(shown.contains(&scene) && shown.contains(&chapter) && !shown.contains(&other), "{shown:?}");
        state.search = "chapter 1".into();
        assert!(state.binder_rows().iter().any(|row| row.id == other), "a chapter whose name matches keeps its scenes");
        state.search.clear();
        state.toggle_collapsed(chapter);
        assert!(!state.binder_rows().iter().any(|row| row.id == scene), "folded");
        state.search = "desert".into();
        assert!(state.binder_rows().iter().any(|row| row.id == scene), "a search opens what it found");
        state.search.clear();
        state.toggle_collapsed(chapter);
        // A chosen row that is gone falls back to a container that is there.
        state.library_selection = Some(Id::new());
        assert!(state.shown_container().is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_folded_library_row_hides_what_is_inside_it_and_stays_folded() {
        let (mut state, dir) = open_state("fold");
        let book = state.binder.manuscript_books()[0].id;
        let inside_book = |state: &StudioState| {
            state.binder_rows().iter().filter(|row| row.id != book).count()
        };
        let before = inside_book(&state);
        assert!(before > 0, "a new project has a chapter in its book");
        assert!(state.binder_rows().iter().any(|row| row.id == book && row.has_children));
        state.toggle_collapsed(book);
        assert!(state.is_collapsed(book));
        assert_eq!(inside_book(&state), 0);
        // Folding is this computer's choice and outlasts closing the project.
        state.save_all();
        let root = state.project_root().unwrap();
        let mut reopened = StudioState::default();
        reopened.open_project_at(root);
        assert!(reopened.is_collapsed(book));
        reopened.toggle_collapsed(book);
        assert_eq!(inside_book(&reopened), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keywords_show_on_cards_and_a_saved_filter_lists_their_scenes() {
        let (mut state, dir) = open_state("keywords");
        let tagged = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let other = state.selected.unwrap();
        state.add_keyword(tagged, "Flashback");
        let card = |state: &StudioState, id: Id| state.binder_rows().into_iter().find(|row| row.id == id);
        let shown = card(&state, tagged).unwrap();
        assert_eq!(shown.keywords.len(), 1);
        assert!(!shown.keywords[0].1.is_empty(), "the keyword has a color");

        state.add_filter(SavedFilter {
            id: Id::new(),
            name: String::new(),
            keywords: vec!["flashback".into()],
            any: false,
            without: vec!["draft".into()],
            entries: Vec::new(),
            status: String::new(),
            text: String::new(),
        });
        assert_eq!(state.binder.filters[0].name, "flashback, not draft", "named for what it looks for");
        assert!(card(&state, tagged).is_some());
        assert!(card(&state, other).is_none(), "the filter shows only its matches");

        // The search's `#keyword` is not looked for in the text.
        state.search = "#flashback writing".into();
        assert!(!state.content_hits().is_empty(), "the words are");
        state.search = "#flashback".into();
        assert!(state.content_hits().is_empty());
        state.search.clear();

        state.keyword_filter = Some("Flashback".into());
        assert!(state.chosen_allows(tagged) && !state.chosen_allows(other));
        // A keyword nested under the one chosen counts, in any alphabet's case.
        state.add_keyword(other, "AÇÃO/fuga");
        state.keyword_filter = Some("ação".into());
        assert!(state.chosen_allows(other) && !state.chosen_allows(tagged));
        state.remove_keyword(other, "AÇÃO/fuga");
        state.keyword_filter = None;
        // A keyword with an empty part is refused, and says why.
        state.add_keyword(other, "act//two");
        assert!(state.status.contains("not a keyword"), "{}", state.status);
        assert!(state.binder.get(other).unwrap().keywords.is_empty());

        // Changing a saved filter keeps its row and shows what it finds now.
        let filter = state.binder.filters[0].id;
        state.edit_filter(filter);
        assert_eq!(state.editing_filter, Some(filter));
        let mut changed = state.binder.filters[0].clone();
        changed.keywords.clear();
        changed.without.clear();
        changed.name = "Everything".into();
        state.add_filter(changed);
        assert_eq!(state.binder.filters.len(), 1);
        assert_eq!(state.binder.filters[0].name, "Everything");
        assert!(card(&state, other).is_some());
        assert_eq!(state.editing_filter, None);

        state.remove_filter(filter);
        assert!(state.binder.filters.is_empty() && state.filter_selection.is_none());
        state.remove_keyword(tagged, "Flashback");
        assert!(state.binder.keywords.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_books_deadline_is_shared_and_gives_a_pace() {
        let (mut state, dir) = open_state("deadline");
        assert_eq!(state.compile_book_pace(), None);
        state.set_compile_book_deadline(Some("not a day".into()));
        assert_eq!(state.compile_book_deadline(), None);
        state.set_compile_book_deadline(Some("2999-01-01".into()));
        assert_eq!(state.compile_book_deadline().as_deref(), Some("2999-01-01"));
        assert!(matches!(state.compile_book_pace(), Some(Pace::Due { .. })));
        state.save_all();
        let root = state.project_root().unwrap();
        let shared = std::fs::read_to_string(root.join("project.json")).unwrap();
        assert!(shared.contains("2999-01-01"), "the deadline goes with the book");
        state.set_compile_book_deadline(None);
        assert_eq!(state.compile_book_pace(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_page_style_is_this_computers_and_stays_in_its_limits() {
        let (mut state, dir) = open_state("page-style");
        assert_eq!(state.page_style(), PageStyle::default());
        let wide = PageStyle { line_length: 900, first_line_indent: 15, ..PageStyle::default() };
        state.set_page_style(wide);
        assert_eq!(state.page_style().line_length, 100);
        assert_eq!(state.page_style().first_line_indent, 15);
        state.save_all();
        let root = state.project_root().unwrap();
        let local = std::fs::read_to_string(root.join(".local").join("settings.json")).unwrap();
        let local: serde_json::Value = serde_json::from_str(&local).unwrap();
        assert_eq!(local["page"]["first_line_indent"], 15);
        let shared = std::fs::read_to_string(root.join("project.json")).unwrap();
        assert!(!shared.contains("first_line_indent"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_scene_added_with_a_chapter_chosen_lands_in_that_chapter() {
        let (mut state, dir) = open_state("add-here");
        let first_scene = state.selected.unwrap();
        state.add_node(NodeKind::Chapter);
        let second_chapter = state.selected.unwrap();
        // Back on a scene of the first chapter, then choose the second one.
        state.select_node(first_scene);
        state.choose_library(second_chapter);
        state.add_node(NodeKind::Scene);
        let added = state.selected.unwrap();
        assert_eq!(state.binder.get(added).unwrap().parent_id, Some(second_chapter));
        assert!(state.binder_rows().iter().any(|row| row.id == added));
        // The next one goes beside it, in the same chapter.
        state.add_node(NodeKind::Scene);
        assert_eq!(state.binder.get(state.selected.unwrap()).unwrap().parent_id, Some(second_chapter));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_item_trashed_from_the_list_leaves_the_open_scene_open() {
        let (mut state, dir) = open_state("trash-item");
        let open = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let other = state.selected.unwrap();
        state.select_node(open);
        state.trash_item(other);
        assert!(state.binder.is_in_trash(other));
        assert_eq!(state.selected, Some(open));
        // Trashing what is open moves the selection with it.
        state.trash_item(open);
        assert!(state.binder.is_in_trash(open));
        assert_eq!(state.selected, state.binder.trash_root().map(|node| node.id));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_book_or_chapter_is_renamed_but_never_to_nothing() {
        let (mut state, dir) = open_state("rename");
        let book = state.binder.manuscript_books()[0].id;
        state.rename_item(book, "  The Glass Cartographer ");
        assert_eq!(state.binder.get(book).unwrap().title, "The Glass Cartographer");
        assert!(!state.has_unsaved());
        state.rename_item(book, "   ");
        assert_eq!(state.binder.get(book).unwrap().title, "The Glass Cartographer");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn each_mode_shows_only_the_panels_it_needs() {
        let with_binder: Vec<_> = WorkspaceMode::ALL.into_iter().filter(|mode| mode.shows_binder()).collect();
        assert_eq!(
            with_binder,
            [WorkspaceMode::Writing, WorkspaceMode::History]
        );
        let with_inspector: Vec<_> = WorkspaceMode::ALL.into_iter().filter(|mode| mode.shows_inspector()).collect();
        // Context shows the node chosen on the mind map and in the outline.
        assert_eq!(with_inspector, [WorkspaceMode::Writing, WorkspaceMode::MindMap, WorkspaceMode::Outliner]);
    }

    #[test]
    fn the_review_panel_keeps_its_tab_while_that_tab_has_items() {
        use ReviewTab::*;
        assert_eq!(ReviewTab::shown(None, &[]), None);
        assert_eq!(ReviewTab::shown(None, &[Notes, Spelling]), Some(Notes));
        assert_eq!(ReviewTab::shown(Some(Spelling), &[Notes, Spelling]), Some(Spelling));
        // Tracking stopped, so its tab went away: the first one left shows.
        assert_eq!(ReviewTab::shown(Some(Changes), &[Spelling]), Some(Spelling));
    }

    #[test]
    fn the_prose_font_is_a_setting_of_this_machine() {
        let (mut state, dir) = open_state("font");
        assert_eq!(state.prose_font(), ProseFont::Serif);
        state.set_prose_font(ProseFont::Sans);
        assert!(state.has_unsaved());
        state.save_all();
        let root = state.project_root().unwrap();
        let local = std::fs::read_to_string(root.join(".local").join("settings.json")).unwrap();
        let local: serde_json::Value = serde_json::from_str(&local).unwrap();
        assert_eq!(local["prose_font"], "sans");
        let shared = std::fs::read_to_string(root.join("project.json")).unwrap();
        assert!(!shared.contains("prose_font"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn escape_closes_the_menu_and_then_the_overlay() {
        let mut state = StudioState::default();
        state.toggle_overlay(FoldedPanel::Binder);
        state.toggle_menu(FrameMenu::Project);
        state.toggle_menu(FrameMenu::Settings);
        state.show_shortcuts = true;
        assert_eq!(state.open_menu, Some(FrameMenu::Settings));
        assert!(state.close_popup());
        assert!(!state.show_shortcuts);
        assert_eq!(state.open_menu, Some(FrameMenu::Settings));
        assert!(state.close_popup());
        assert_eq!(state.overlay, Some(FoldedPanel::Binder));
        assert!(state.close_popup());
        assert!(!state.close_popup());
        // A second click on the same menu closes it.
        state.toggle_menu(FrameMenu::Goals);
        state.toggle_menu(FrameMenu::Goals);
        assert_eq!(state.open_menu, None);
    }

    #[test]
    fn the_binder_changed_outside_is_read_again_and_kept_versions_are_listed() {
        let (mut state, dir) = open_state("lists");
        let root = state.project_root().unwrap();
        state.update_selected_title("Mine".into());
        state.set_daily_goal(700);
        state.add_plotline();
        state.save_all();
        // What the program wrote itself is not taken for a change outside.
        state.status.clear();
        state.check_files_on_disk();
        assert!(state.status.is_empty(), "{}", state.status);

        // Changed outside, nothing unsaved: the window shows the change.
        let binder = root.join("binder.json");
        let scene = state.selected.unwrap();
        let file = open_scene_file(&state);
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::write(&file, text.replace("\ntitle: Mine\n", "\ntitle: From git\n")).unwrap();
        let listed = std::fs::read_to_string(&binder).unwrap();
        std::fs::write(&binder, listed.replace("\"kind\"", "\"x\":1,\"kind\"")).unwrap();
        state.check_files_on_disk();
        assert!(state.status.contains("read again"), "{}", state.status);
        assert_eq!(state.binder.get(scene).unwrap().title, "From git");
        assert_eq!(state.selected, Some(scene));

        // With something unsaved nothing is replaced.
        state.apply_editor_text("Typing, not saved.".into());
        let listed = std::fs::read_to_string(&binder).unwrap();
        std::fs::write(&binder, listed.replace("\"x\":1", "\"x\":2")).unwrap();
        state.check_files_on_disk();
        assert!(state.status.contains("changed outside"), "{}", state.status);
        assert_eq!(state.editor_text, "Typing, not saved.");

        // A scene changed outside and saved over is listed, and comes back.
        let outside = format!("{}\nOutside.\n", std::fs::read_to_string(&file).unwrap());
        std::fs::write(&file, &outside).unwrap();
        state.flush_editor();
        let kept = state.kept_files();
        let (item, mine) = kept.iter().find(|(item, _)| item.kept_as.starts_with("manuscript/")).unwrap();
        assert!(*mine);
        let earlier = Earlier::Kept(item.kept_as.clone());
        state.compare_with_revision(earlier.clone());
        assert!(state.snapshot_comparison().unwrap().is_ok());
        state.restore_revision(&earlier);
        assert!(state.editor_text.contains("Outside."), "{}", state.editor_text);
        let kept_as = item.kept_as.clone();
        state.request_kept_removal(kept_as.clone());
        assert!(root.join(&kept_as).exists());
        state.request_kept_removal(kept_as.clone());
        assert!(!root.join(&kept_as).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_scene_changed_outside_is_read_again_unless_the_writer_is_typing() {
        let (mut state, dir) = open_state("outside");
        state.apply_editor_text("Written here.".into());
        state.flush_editor();
        let file = open_scene_file(&state);
        let scene = state.selected.unwrap();

        // Changed outside, nothing unsaved: the window shows the change.
        let text = std::fs::read_to_string(&file).unwrap().replace("Written here.", "Written in git.");
        std::fs::write(&file, text.replace("\ntitle: Scene 1\n", "\ntitle: Pulled\n")).unwrap();
        state.check_files_on_disk();
        assert_eq!(state.editor_text, "Written in git.");
        assert_eq!(state.binder.get(scene).unwrap().title, "Pulled");
        assert!(state.status.contains("read again"), "{}", state.status);

        // Changed outside while the writer has unsaved typing: nothing is
        // replaced, and the save keeps both.
        state.apply_editor_text("Typing, not saved.".into());
        let outside = std::fs::read_to_string(&file).unwrap().replace("Written in git.", "Changed again outside.");
        std::fs::write(&file, &outside).unwrap();
        state.check_files_on_disk();
        assert_eq!(state.editor_text, "Typing, not saved.");
        assert!(state.status.contains("changed outside"), "{}", state.status);
        state.flush_editor();
        assert!(std::fs::read_to_string(&file).unwrap().contains("Typing, not saved."));
        assert!(state.status.contains(".conflict-"), "{}", state.status);
        let kept = std::fs::read_dir(file.parent().unwrap())
            .unwrap()
            .flatten()
            .find(|entry| entry.file_name().to_string_lossy().contains(".conflict-"))
            .unwrap();
        assert_eq!(std::fs::read_to_string(kept.path()).unwrap(), outside);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn markdown_files_come_in_as_scenes_where_a_new_scene_would_go() {
        let (mut state, dir) = open_state("import");
        let chapter = state.binder.get(state.selected.unwrap()).unwrap().parent_id;
        let files = dir.join("incoming");
        std::fs::create_dir_all(&files).unwrap();
        let good = files.join("arrival.md");
        std::fs::write(&good, "---\ngoal: Land\ntags:\n- \"#harbor/night\"\n- Flashback\n---\n\nThe ship came in.\n").unwrap();
        let broken = files.join("broken.md");
        std::fs::write(&broken, "---\ntitle: never closed\n").unwrap();

        state.import_markdown_files(vec![good, broken]);
        let scene = state.binder.get(state.selected.unwrap()).unwrap().clone();
        assert_eq!((scene.title.as_str(), scene.parent_id), ("arrival", chapter));
        assert_eq!(state.editor_text, "The ship came in.");
        assert_eq!(state.scene_beats(scene.id).goal, "Land");
        // Its tags are keywords, each with a color, and saved.
        assert_eq!(scene.keywords, ["harbor/night", "Flashback"]);
        assert!(!state.binder.keyword_color("flashback").is_empty());
        let saved = std::fs::read_to_string(state.project_root().unwrap().join("binder.json")).unwrap();
        assert!(saved.contains("harbor/night"), "{saved}");
        assert!(state.status.starts_with("Imported 1 scenes. Not imported: broken:"), "{}", state.status);

        let parent = dir.join("exports");
        std::fs::create_dir_all(&parent).unwrap();
        state.export_markdown_folder(parent.clone());
        // The two scenes. A new project's notes are two empty folders.
        assert!(state.status.starts_with("Exported 2 files"), "{}", state.status);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn problems_found_on_opening_are_reported_and_a_missing_scene_is_marked() {
        let (state, dir) = open_state("problems");
        let scene = state.selected.unwrap();
        let file = open_scene_file(&state);
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        drop(state);
        std::fs::remove_file(&file).unwrap();

        let mut state = StudioState::default();
        state.open_project_at(root);
        assert!(state.status.contains("needed attention"), "{}", state.status);
        assert_eq!(state.open_problems().len(), 1);
        assert!(state.is_missing(scene));
        state.select_node(scene);
        assert!(state.status.contains("not written over"), "{}", state.status);
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn git(dir: &std::path::Path, args: &[&str]) {
        let done = std::process::Command::new("git").current_dir(dir).args(args).output().unwrap();
        assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
    }

    #[test]
    fn the_window_syncs_reopens_and_marks_what_both_machines_changed() {
        let (mut state, dir) = open_state("sync");
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let remote = dir.join("remote.git");
        git(&dir, &["init", "-q", "--bare", "--initial-branch=main", &remote.to_string_lossy()]);
        assert!(state.sync_status().is_none());
        state.sync_address = remote.to_string_lossy().into_owned();
        state.connect_sync();
        for (key, value) in [("user.name", "Test Writer"), ("user.email", "test@example.com")] {
            git(&root, &["config", key, value]);
        }
        state.apply_editor_text("One.\nTwo.\nThree.".into());
        let folder = state.begin_sync().unwrap();
        assert!(state.syncing && state.begin_sync().is_none());
        state.finish_sync(git::sync(&folder));
        assert!(!state.syncing);
        assert_eq!(state.status, "Synced: sent this machine's changes.");
        assert!(state.sync_status().is_some());

        // Another machine changes the same paragraph and sends it.
        let other = dir.join("other.writingstudio");
        git(&dir, &["clone", "-q", &remote.to_string_lossy(), &other.to_string_lossy()]);
        for (key, value) in [("user.name", "Other"), ("user.email", "other@example.com")] {
            git(&other, &["config", key, value]);
        }
        let scene = state.selected.unwrap();
        {
            let store = ProjectStore::open(&other).unwrap();
            store.save_document(&writing_studio_document::Document::from_plain_text(scene, "One.\nTwo, there.\nThree.")).unwrap();
        }
        git::sync(&other).unwrap();

        state.apply_editor_text("One.\nTwo, here.\nThree.".into());
        let folder = state.begin_sync().unwrap();
        state.finish_sync(git::sync(&folder));
        assert!(state.status.contains("History lists them"), "{}", state.status);
        assert_eq!(state.selected, Some(scene));
        assert_eq!(state.editor_text, "One.\nTwo, here.\nTwo, there.\nThree.");
        assert!(state.is_tracking());
        assert_eq!(state.tracked_changes().len(), 1);
        assert_eq!(state.sync_conflicts.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_scene_can_be_compared_with_and_restored_from_an_earlier_sync() {
        let (mut state, dir) = open_state("revisions");
        let root = state.store.as_ref().unwrap().borrow().root.clone();
        let remote = dir.join("remote.git");
        git(&dir, &["init", "-q", "--bare", "--initial-branch=main", &remote.to_string_lossy()]);
        state.sync_address = remote.to_string_lossy().into_owned();
        state.connect_sync();
        for (key, value) in [("user.name", "Test Writer"), ("user.email", "test@example.com")] {
            git(&root, &["config", key, value]);
        }
        for text in ["First draft.", "Second draft."] {
            state.apply_editor_text(text.into());
            let folder = state.begin_sync().unwrap();
            state.finish_sync(git::sync(&folder));
        }
        let revisions = state.scene_revisions();
        assert_eq!(revisions.len(), 2);
        assert!(revisions[0].message.contains("Scene 1"), "{revisions:?}");

        let older = revisions[1].commit.clone();
        state.compare_with_revision(Earlier::Sync(older.clone()));
        let comparison = state.snapshot_comparison().unwrap().unwrap();
        assert!(comparison.older.starts_with("Synced "), "{}", comparison.older);
        assert!(!comparison.diff.is_unchanged());

        state.restore_revision(&Earlier::Sync(older));
        assert_eq!(state.editor_text, "First draft.");
        state.undo();
        assert_eq!(state.editor_text, "Second draft.");
        state.restore_revision(&Earlier::Sync("not-a-commit".into()));
        assert!(state.status.contains("not a commit"), "{}", state.status);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn choosing_another_document_closes_the_comparison() {
        let (mut state, dir) = open_state("compare-select");
        state.apply_editor_text("First draft.".into());
        state.snapshot();
        let snapshot = state.snapshots_for_selected()[0].id;
        state.compare_snapshot(snapshot);
        assert!(state.snapshot_comparison().is_some());

        let current = state.selected.unwrap();
        state.select_node(current);
        assert_eq!(state.compare_snapshot, Some(snapshot));

        let other = state
            .binder
            .nodes
            .iter()
            .find(|node| node.id != current && node.kind == NodeKind::Chapter)
            .map(|node| node.id)
            .unwrap();
        state.select_node(other);
        assert!(state.compare_snapshot.is_none());
        assert!(state.snapshot_comparison().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
