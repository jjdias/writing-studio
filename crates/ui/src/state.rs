use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use writing_studio_characters::Cast;
use writing_studio_corkboard::CorkGroup;
use writing_studio_document::{
    accept_change, compare_documents, reject_change, tracked_changes, Document, DocumentDiff,
    NoteKind, ReviewItem, ReviewKind, ReviewTarget, TrackedChange,
};
use writing_studio_core::{autosave_due, autosave_wait, local_day, Id, Timestamp, PRODUCT_NAME};
use writing_studio_editor::EditorSession;
use writing_studio_export::{compile_docx, compile_markdown, CompileInput};
use writing_studio_manuscript::book_outline_words;
use writing_studio_plotting::PlotGrid;
use writing_studio_research::ResearchLibrary;
use writing_studio_project::{
    format_elapsed, normalize_binder_color, BinderNode, BinderTree, NodeKind, PanelLayout,
    ProjectMeta, StoredSession, Theme,
};
use writing_studio_search::ManuscriptIndex;
use writing_studio_spellcheck::{
    available_dictionaries, name_words, normalize_word, replace_in_document, DictionaryInfo,
    Speller, UnknownWord,
};
use writing_studio_storage::{BackupInfo, ProjectStore, SnapshotInfo};
use writing_studio_worldbuilding::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceMode {
    Writing,
    Corkboard,
    Outliner,
    Plot,
    Characters,
    World,
    Research,
    History,
}

impl WorkspaceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Writing => "Writing",
            Self::Corkboard => "Corkboard",
            Self::Outliner => "Outliner",
            Self::Plot => "Plot",
            Self::Characters => "Characters",
            Self::World => "World",
            Self::Research => "Research",
            Self::History => "History",
        }
    }

    pub fn view_title(self) -> &'static str {
        match self {
            Self::Writing => "Explorer",
            Self::Corkboard => "Corkboard",
            Self::Outliner => "Outliner",
            Self::Plot => "Plot",
            Self::Characters => "Characters",
            Self::World => "World",
            Self::Research => "Research",
            Self::History => "History",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldSection {
    #[default]
    Locations,
    Cultures,
    Lore,
    Timeline,
}

impl WorldSection {
    pub const ALL: [WorldSection; 4] = [
        WorldSection::Locations,
        WorldSection::Cultures,
        WorldSection::Lore,
        WorldSection::Timeline,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Locations => "Locations",
            Self::Cultures => "Cultures",
            Self::Lore => "Lore",
            Self::Timeline => "Timeline",
        }
    }

    pub fn add_label(self) -> &'static str {
        match self {
            Self::Locations => "Add location",
            Self::Cultures => "Add culture",
            Self::Lore => "Add lore",
            Self::Timeline => "Add event",
        }
    }
}

/// Which project-level bible record a scene link belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BibleLink {
    Character,
    Location,
    Culture,
    Lore,
    Timeline,
    Research,
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

#[derive(Clone)]
pub struct StudioState {
    pub product_name: &'static str,
    pub store: Option<Rc<RefCell<ProjectStore>>>,
    pub meta: Option<ProjectMeta>,
    pub binder: BinderTree,
    pub selected: Option<Id>,
    pub session: Option<EditorSession>,
    pub editor_text: String,
    pub editor_html: String,
    pub editor_epoch: u64,
    pub search: String,
    pub manuscript_index: Rc<RefCell<ManuscriptIndex>>,
    pub cork_query: String,
    pub cork_group: CorkGroup,
    pub plot: PlotGrid,
    pub plot_dirty: bool,
    pub confirm_plotline: Option<Id>,
    pub cast: Cast,
    pub cast_dirty: bool,
    pub selected_character: Option<Id>,
    pub confirm_character: Option<Id>,
    pub world: World,
    pub world_dirty: bool,
    pub world_section: WorldSection,
    pub selected_world: Option<Id>,
    pub confirm_world: Option<Id>,
    pub library: ResearchLibrary,
    pub library_dirty: bool,
    pub selected_research: Option<Id>,
    pub confirm_research: Option<Id>,
    pub mode: WorkspaceMode,
    pub focus_mode: bool,
    pub status: String,
    pub new_title: String,
    pub confirm_backup: Option<PathBuf>,
    pub compare_snapshot: Option<Id>,
    pub compare_against: Option<Id>,
    pub compare_changes_only: bool,
    pub show_resolved_notes: bool,
    pub tracking: Option<Tracking>,
    /// The dictionary in use. `None` while spellcheck is off or before a project is open.
    pub speller: Option<Rc<Speller>>,
    /// The unknown word whose suggestions are on screen.
    pub spell_focus: Option<String>,
    pub show_project_words: bool,
    pub binder_dirty: bool,
    pub last_edit: Option<Instant>,
    pub session_started: Option<Instant>,
    pub session_baseline: u32,
    pub session_shown_secs: u64,
    pub settings_dirty: bool,
    pub dragging: Option<Id>,
    pub drop_target: Option<Id>,
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
            editor_html: String::new(),
            editor_epoch: 0,
            search: String::new(),
            manuscript_index: Rc::new(RefCell::new(ManuscriptIndex::new())),
            cork_query: String::new(),
            cork_group: CorkGroup::Manuscript,
            plot: PlotGrid::new(),
            plot_dirty: false,
            confirm_plotline: None,
            cast: Cast::new(),
            cast_dirty: false,
            selected_character: None,
            confirm_character: None,
            world: World::new(),
            world_dirty: false,
            world_section: WorldSection::Locations,
            selected_world: None,
            confirm_world: None,
            library: ResearchLibrary::new(),
            library_dirty: false,
            selected_research: None,
            confirm_research: None,
            mode: WorkspaceMode::Writing,
            focus_mode: false,
            status: "Offline · Local-first".into(),
            new_title: "Untitled Novel".into(),
            confirm_backup: None,
            compare_snapshot: None,
            compare_against: None,
            compare_changes_only: false,
            show_resolved_notes: false,
            tracking: None,
            speller: None,
            spell_focus: None,
            show_project_words: false,
            binder_dirty: false,
            last_edit: None,
            session_started: None,
            session_baseline: 0,
            session_shown_secs: 0,
            settings_dirty: false,
            dragging: None,
            drop_target: None,
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

    /// The book the binder, corkboard, outliner, and search are limited to.
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
        let root: PathBuf = folder.join(format!("{}.writingstudio", sanitize(title)));
        match ProjectStore::create(&root, title) {
            Ok(store) => self.attach_store(store, "Created project and saved locally."),
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
            store.load_cast(),
            store.load_world(),
            store.load_library(),
        ) {
            (Ok(meta), Ok(binder), Ok(plot), Ok(cast), Ok(world), Ok(library)) => {
                self.meta = Some(meta);
                self.binder = binder;
                self.plot = plot;
                self.plot_dirty = false;
                self.confirm_plotline = None;
                self.cast = cast;
                self.cast_dirty = false;
                self.selected_character = None;
                self.confirm_character = None;
                self.world = world;
                self.world_dirty = false;
                self.selected_world = None;
                self.confirm_world = None;
                self.library = library;
                self.library_dirty = false;
                self.selected_research = None;
                self.confirm_research = None;
                self.store = Some(Rc::new(RefCell::new(store)));
                if self.absorb_binder_research() {
                    self.status = status.into();
                }
                if let Some(id) = self.opening_scene() {
                    self.select_node(id);
                }
                self.reindex_manuscript();
                self.begin_writing_session();
                self.load_speller();
            }
            (Err(err), _, _, _, _, _)
            | (_, Err(err), _, _, _, _)
            | (_, _, Err(err), _, _, _)
            | (_, _, _, Err(err), _, _)
            | (_, _, _, _, Err(err), _)
            | (_, _, _, _, _, Err(err)) => {
                self.status = format!("Project load failed: {err}");
            }
        }
    }

    pub fn select_node(&mut self, id: Id) {
        self.flush_editor();
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
            self.editor_html.clear();
            self.editor_epoch += 1;
        }
        self.load_tracking();
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
    fn accepted_words(&self) -> HashSet<String> {
        let mut words = name_words(
            self.cast
                .characters
                .iter()
                .map(|character| character.name.as_str())
                .chain(self.world.locations.iter().map(|place| place.name.as_str()))
                .chain(self.world.cultures.iter().map(|culture| culture.name.as_str())),
        );
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
        self.editor_html = session.html();
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

    pub fn same_editor_html(&self, html: &str) -> bool {
        let Some(session) = &self.session else {
            return true;
        };
        let mut next = session.document().clone();
        next.replace_html(html);
        next.to_html() == session.html()
    }

    pub fn apply_editor_html(&mut self, html: String) {
        if self.same_editor_html(&html) {
            return;
        }
        if let Some(session) = &mut self.session {
            session.apply_html(&html);
            self.editor_text = session.plain_text();
            if session.is_dirty() {
                self.note_edit();
                self.note_session_for_search();
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
    }

    pub fn save_all(&mut self) {
        self.flush_editor();
        if self.binder_dirty && self.store.is_some() && !self.persist_binder() {
            return;
        }
        if self.plot_dirty && self.store.is_some() && !self.persist_plot_now() {
            return;
        }
        if self.cast_dirty && self.store.is_some() && !self.persist_cast_now() {
            return;
        }
        if self.world_dirty && self.store.is_some() && !self.persist_world_now() {
            return;
        }
        if self.library_dirty && self.store.is_some() && !self.persist_library_now() {
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
        self.cast_dirty = false;
        self.world_dirty = false;
        self.library_dirty = false;
        self.settings_dirty = false;
        self.last_edit = None;
        self.status = "All changes saved locally.".into();
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

    pub fn has_unsaved(&self) -> bool {
        self.settings_dirty
            || self.binder_dirty
            || self.plot_dirty
            || self.cast_dirty
            || self.world_dirty
            || self.library_dirty
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
        self.manuscript_index.borrow().search(&self.search)
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
        self.manuscript_index.borrow_mut().set_pending(id, text);
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

    pub fn prepare_export(&mut self, format: CompiledFormat) -> Option<ExportFile> {
        self.save_all();
        let Some(store) = self.store.clone() else {
            self.status = "Open a project to compile.".into();
            return None;
        };
        let documents = match store.borrow().load_all_documents() {
            Ok(documents) => documents,
            Err(err) => {
                self.status = format!("Could not load manuscript for compile: {err}");
                return None;
            }
        };
        let Some(book_id) = self.compile_book_id() else {
            self.status = "Add a book before compiling.".into();
            return None;
        };
        let title = self
            .binder
            .get(book_id)
            .map(|node| node.title.clone())
            .unwrap_or_else(|| self.project_title());
        let include_title_page = self
            .meta
            .as_ref()
            .map(|meta| meta.settings.compile_title_page)
            .unwrap_or(true);
        let input = CompileInput {
            title: &title,
            include_title_page,
            book_id: Some(book_id),
            tree: &self.binder,
            documents: &documents,
        };
        let file_stem = sanitize(&title);
        match format {
            CompiledFormat::Markdown => Some(ExportFile {
                bytes: compile_markdown(&input).into_bytes(),
                file_name: format!("{file_stem}.md"),
                filter_name: "Markdown",
                extension: "md",
                kind_label: "Markdown",
            }),
            CompiledFormat::Docx => match compile_docx(&input) {
                Ok(bytes) => Some(ExportFile {
                    bytes,
                    file_name: format!("{file_stem}.docx"),
                    filter_name: "Word",
                    extension: "docx",
                    kind_label: "DOCX",
                }),
                Err(err) => {
                    self.status = format!("Could not compile DOCX: {err}");
                    None
                }
            },
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
    }

    /// Compare the open snapshot with another snapshot of the same document.
    /// `None` compares it with the current text.
    pub fn set_compare_against(&mut self, other: Option<Id>) {
        self.compare_against = other.filter(|id| Some(*id) != self.compare_snapshot);
    }

    pub fn close_comparison(&mut self) {
        self.compare_snapshot = None;
        self.compare_against = None;
    }

    /// Nothing is written: both sides are read, and the live document and the
    /// snapshots stay as they are.
    pub fn snapshot_comparison(&self) -> Option<Result<SnapshotComparison, String>> {
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
        match restored {
            Ok(document) => {
                let id = document.id;
                let text = document.to_plain_text();
                let indexed = self.manuscript_index.borrow_mut().upsert(id, &text);
                let session = EditorSession::new(document);
                self.sync_editor_from_session(&session);
                self.session = Some(session);
                self.status = match indexed {
                    Ok(()) => "Snapshot restored locally. Previous text was saved as a pre-restore snapshot.".into(),
                    Err(err) => format!("Snapshot restored, but search index failed: {err}"),
                };
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
        let (meta, binder, plot, cast, world, library) = {
            let store = store.borrow();
            (
                store.load_meta(),
                store.load_binder(),
                store.load_plot(),
                store.load_cast(),
                store.load_world(),
                store.load_library(),
            )
        };
        match (meta, binder, plot, cast, world, library) {
            (Ok(meta), Ok(binder), Ok(plot), Ok(cast), Ok(world), Ok(library)) => {
                let previous = self.selected;
                self.meta = Some(meta);
                self.binder = binder;
                self.plot = plot;
                self.plot_dirty = false;
                self.confirm_plotline = None;
                self.cast = cast;
                self.cast_dirty = false;
                self.selected_character = self
                    .selected_character
                    .filter(|id| self.cast.characters.iter().any(|character| character.id == *id));
                self.confirm_character = None;
                self.world = world;
                self.world_dirty = false;
                self.selected_world = self.selected_world.filter(|id| self.world_contains(*id));
                self.confirm_world = None;
                self.library = library;
                self.library_dirty = false;
                self.selected_research = self
                    .selected_research
                    .filter(|id| self.library.items.iter().any(|item| item.id == *id));
                self.confirm_research = None;
                let moved = self.absorb_binder_research();
                self.session = None;
                self.editor_text.clear();
                self.editor_html.clear();
                self.editor_epoch += 1;
                if moved {
                    self.status = status.into();
                }
                if let Some(id) = previous.filter(|id| self.binder.get(*id).is_some()) {
                    self.select_node(id);
                } else if let Some(id) = self.opening_scene() {
                    self.select_node(id);
                }
                self.reindex_manuscript();
                self.begin_writing_session();
                self.load_speller();
            }
            (Err(err), _, _, _, _, _)
            | (_, Err(err), _, _, _, _)
            | (_, _, Err(err), _, _, _)
            | (_, _, _, Err(err), _, _)
            | (_, _, _, _, Err(err), _)
            | (_, _, _, _, _, Err(err)) => {
                self.status = format!("Backup restored, but reload failed: {err}");
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(session) = &mut self.session {
            if session.undo() {
                let html = session.html();
                let text = session.plain_text();
                self.editor_html = html;
                self.editor_text = text;
                self.editor_epoch = self.editor_epoch.saturating_add(1);
                self.note_edit();
                self.note_session_for_search();
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(session) = &mut self.session {
            if session.redo() {
                let html = session.html();
                let text = session.plain_text();
                self.editor_html = html;
                self.editor_text = text;
                self.editor_epoch = self.editor_epoch.saturating_add(1);
                self.note_edit();
                self.note_session_for_search();
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
            NodeKind::Part => "New part".into(),
            NodeKind::Book => format!("Book {}", self.binder.manuscript_books().len() + 1),
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

    pub fn add_plotline(&mut self) {
        self.confirm_plotline = None;
        let mut grid = self.plot.clone();
        grid.add_line();
        self.store_plot(grid, true);
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
            self.status = "That scene is no longer in the binder.".into();
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

    pub fn add_character(&mut self) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        let id = cast.add_character();
        self.selected_character = Some(id);
        self.store_cast(cast, true);
    }

    pub fn select_character(&mut self, id: Id) {
        if self.cast.characters.iter().any(|character| character.id == id) {
            self.confirm_character = None;
            self.selected_character = Some(id);
        }
    }

    pub fn set_character_name(&mut self, id: Id, name: String) {
        self.edit_character(|cast| cast.set_name(id, name));
    }

    pub fn set_character_role(&mut self, id: Id, role: String) {
        self.edit_character(|cast| cast.set_role(id, role));
    }

    pub fn set_character_summary(&mut self, id: Id, summary: String) {
        self.edit_character(|cast| cast.set_summary(id, summary));
    }

    pub fn remove_character(&mut self, id: Id) {
        if self.confirm_character != Some(id) {
            self.confirm_character = Some(id);
            self.status = "Click delete again to remove this character, their motivations, and their relationships.".into();
            return;
        }
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if !cast.remove_character(id) {
            return;
        }
        if self.selected_character == Some(id) {
            self.selected_character = cast.characters_in_order().first().map(|character| character.id);
        }
        self.store_cast(cast, true);
    }

    pub fn add_motivation(&mut self, character_id: Id) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.add_motivation(character_id).is_some() {
            self.store_cast(cast, true);
        }
    }

    pub fn set_motivation_body(&mut self, id: Id, body: String) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.set_motivation(id, body) {
            self.store_cast(cast, false);
        }
    }

    pub fn shift_motivation(&mut self, id: Id, earlier: bool) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.shift_motivation(id, earlier) {
            self.store_cast(cast, true);
        }
    }

    pub fn remove_motivation(&mut self, id: Id) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.remove_motivation(id) {
            self.store_cast(cast, true);
        }
    }

    pub fn add_relationship(&mut self, from_id: Id) {
        self.confirm_character = None;
        let other = self
            .cast
            .characters_in_order()
            .into_iter()
            .map(|character| character.id)
            .find(|id| *id != from_id);
        let Some(to_id) = other else {
            self.status = "Add another character before linking a relationship.".into();
            return;
        };
        let mut cast = self.cast.clone();
        if cast.add_relationship(from_id, to_id).is_some() {
            self.store_cast(cast, true);
        }
    }

    pub fn set_relationship_kind(&mut self, id: Id, kind: String) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.set_relationship_kind(id, kind) {
            self.store_cast(cast, false);
        }
    }

    pub fn set_relationship_target(&mut self, id: Id, to_id: Id) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.set_relationship_target(id, to_id) {
            self.store_cast(cast, true);
        }
    }

    pub fn link_bible_scene(&mut self, kind: BibleLink, owner: Id, scene_id: Id) {
        match kind {
            BibleLink::Character => {
                let mut cast = self.cast.clone();
                if cast.link_scene(owner, scene_id) {
                    self.store_cast(cast, true);
                }
            }
            BibleLink::Location => {
                let mut world = self.world.clone();
                if world.link_location_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Culture => {
                let mut world = self.world.clone();
                if world.link_culture_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Lore => {
                let mut world = self.world.clone();
                if world.link_lore_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Timeline => {
                let mut world = self.world.clone();
                if world.link_event_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Research => {
                let mut library = self.library.clone();
                if library.link_scene(owner, scene_id) {
                    self.store_library(library, true);
                }
            }
        }
    }

    pub fn unlink_bible_scene(&mut self, kind: BibleLink, owner: Id, scene_id: Id) {
        match kind {
            BibleLink::Character => {
                let mut cast = self.cast.clone();
                if cast.unlink_scene(owner, scene_id) {
                    self.store_cast(cast, true);
                }
            }
            BibleLink::Location => {
                let mut world = self.world.clone();
                if world.unlink_location_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Culture => {
                let mut world = self.world.clone();
                if world.unlink_culture_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Lore => {
                let mut world = self.world.clone();
                if world.unlink_lore_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Timeline => {
                let mut world = self.world.clone();
                if world.unlink_event_scene(owner, scene_id) {
                    self.store_world(world, true);
                }
            }
            BibleLink::Research => {
                let mut library = self.library.clone();
                if library.unlink_scene(owner, scene_id) {
                    self.store_library(library, true);
                }
            }
        }
    }

    pub fn bible_scene_ids(&self, kind: BibleLink, owner: Id) -> Vec<Id> {
        match kind {
            BibleLink::Character => self
                .cast
                .characters
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
            BibleLink::Location => self
                .world
                .locations
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
            BibleLink::Culture => self
                .world
                .cultures
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
            BibleLink::Lore => self
                .world
                .lore
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
            BibleLink::Timeline => self
                .world
                .events
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
            BibleLink::Research => self
                .library
                .items
                .iter()
                .find(|item| item.id == owner)
                .map(|item| item.scene_ids.clone())
                .unwrap_or_default(),
        }
    }

    pub fn remove_relationship(&mut self, id: Id) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if cast.remove_relationship(id) {
            self.store_cast(cast, true);
        }
    }

    fn edit_character(&mut self, edit: impl FnOnce(&mut Cast) -> bool) {
        self.confirm_character = None;
        let mut cast = self.cast.clone();
        if edit(&mut cast) {
            self.store_cast(cast, false);
        }
    }

    fn store_cast(&mut self, cast: Cast, immediate: bool) {
        if !immediate {
            self.cast = cast;
            self.cast_dirty = true;
            self.note_edit();
            return;
        }
        let Some(store) = &self.store else {
            self.cast = cast;
            self.cast_dirty = false;
            return;
        };
        match store.borrow().persist_cast(&cast) {
            Ok(()) => {
                self.cast = cast;
                self.cast_dirty = false;
                self.status = "Characters saved locally.".into();
            }
            Err(err) => self.status = format!("Could not save characters: {err}"),
        }
    }

    fn persist_cast_now(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_cast(&self.cast) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save characters: {err}");
                false
            }
        }
    }

    pub fn set_world_section(&mut self, section: WorldSection) {
        self.world_section = section;
        self.confirm_world = None;
    }

    pub fn select_world_item(&mut self, id: Id) {
        if self.world_contains(id) {
            self.confirm_world = None;
            self.selected_world = Some(id);
        }
    }

    pub fn add_world_item(&mut self) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        let id = match self.world_section {
            WorldSection::Locations => world.add_location(),
            WorldSection::Cultures => world.add_culture(),
            WorldSection::Lore => world.add_lore(),
            WorldSection::Timeline => world.add_event(),
        };
        self.selected_world = Some(id);
        self.store_world(world, true);
    }

    pub fn remove_world_item(&mut self, id: Id) {
        if self.confirm_world != Some(id) {
            self.confirm_world = Some(id);
            self.status = "Click delete again to remove this entry. Links to it are cleared.".into();
            return;
        }
        self.confirm_world = None;
        let mut world = self.world.clone();
        let removed = match self.world_section {
            WorldSection::Locations => world.remove_location(id),
            WorldSection::Cultures => world.remove_culture(id),
            WorldSection::Lore => world.remove_lore(id),
            WorldSection::Timeline => world.remove_event(id),
        };
        if !removed {
            return;
        }
        if self.selected_world == Some(id) {
            self.selected_world = self.first_world_id(&world);
        }
        self.store_world(world, true);
    }

    pub fn set_location_name(&mut self, id: Id, name: String) {
        self.edit_world(|world| world.set_location_name(id, name));
    }

    pub fn set_location_summary(&mut self, id: Id, summary: String) {
        self.edit_world(|world| world.set_location_summary(id, summary));
    }

    pub fn set_location_culture(&mut self, id: Id, culture_id: Option<Id>) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if world.set_location_culture(id, culture_id) {
            self.store_world(world, true);
        }
    }

    pub fn set_culture_name(&mut self, id: Id, name: String) {
        self.edit_world(|world| world.set_culture_name(id, name));
    }

    pub fn set_culture_summary(&mut self, id: Id, summary: String) {
        self.edit_world(|world| world.set_culture_summary(id, summary));
    }

    pub fn set_lore_title(&mut self, id: Id, title: String) {
        self.edit_world(|world| world.set_lore_title(id, title));
    }

    pub fn set_lore_body(&mut self, id: Id, body: String) {
        self.edit_world(|world| world.set_lore_body(id, body));
    }

    pub fn set_lore_location(&mut self, id: Id, location_id: Option<Id>) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if world.set_lore_location(id, location_id) {
            self.store_world(world, true);
        }
    }

    pub fn set_lore_culture(&mut self, id: Id, culture_id: Option<Id>) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if world.set_lore_culture(id, culture_id) {
            self.store_world(world, true);
        }
    }

    pub fn set_event_title(&mut self, id: Id, title: String) {
        self.edit_world(|world| world.set_event_title(id, title));
    }

    pub fn set_event_when(&mut self, id: Id, when_label: String) {
        self.edit_world(|world| world.set_event_when(id, when_label));
    }

    pub fn set_event_summary(&mut self, id: Id, summary: String) {
        self.edit_world(|world| world.set_event_summary(id, summary));
    }

    pub fn set_event_location(&mut self, id: Id, location_id: Option<Id>) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if world.set_event_location(id, location_id) {
            self.store_world(world, true);
        }
    }

    pub fn shift_world_event(&mut self, id: Id, earlier: bool) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if world.shift_event(id, earlier) {
            self.store_world(world, true);
        }
    }

    fn edit_world(&mut self, edit: impl FnOnce(&mut World) -> bool) {
        self.confirm_world = None;
        let mut world = self.world.clone();
        if edit(&mut world) {
            self.store_world(world, false);
        }
    }

    fn store_world(&mut self, world: World, immediate: bool) {
        if !immediate {
            self.world = world;
            self.world_dirty = true;
            self.note_edit();
            return;
        }
        let Some(store) = &self.store else {
            self.world = world;
            self.world_dirty = false;
            return;
        };
        match store.borrow().persist_world(&world) {
            Ok(()) => {
                self.world = world;
                self.world_dirty = false;
                self.status = "World saved locally.".into();
            }
            Err(err) => self.status = format!("Could not save world: {err}"),
        }
    }

    fn persist_world_now(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_world(&self.world) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save world: {err}");
                false
            }
        }
    }

    fn world_contains(&self, id: Id) -> bool {
        self.world.locations.iter().any(|item| item.id == id)
            || self.world.cultures.iter().any(|item| item.id == id)
            || self.world.lore.iter().any(|item| item.id == id)
            || self.world.events.iter().any(|item| item.id == id)
    }

    fn first_world_id(&self, world: &World) -> Option<Id> {
        match self.world_section {
            WorldSection::Locations => world.locations_in_order().first().map(|item| item.id),
            WorldSection::Cultures => world.cultures_in_order().first().map(|item| item.id),
            WorldSection::Lore => world.lore_in_order().first().map(|item| item.id),
            WorldSection::Timeline => world.events_in_order().first().map(|item| item.id),
        }
    }

    pub fn add_research_item(&mut self) {
        self.confirm_research = None;
        let mut library = self.library.clone();
        let id = library.add_item();
        self.selected_research = Some(id);
        self.store_library(library, true);
    }

    pub fn select_research_item(&mut self, id: Id) {
        if self.library.items.iter().any(|item| item.id == id) {
            self.confirm_research = None;
            self.selected_research = Some(id);
        }
    }

    pub fn set_research_title(&mut self, id: Id, title: String) {
        self.edit_library(|library| library.set_title(id, title));
    }

    pub fn set_research_notes(&mut self, id: Id, notes: String) {
        self.edit_library(|library| library.set_notes(id, notes));
    }

    pub fn remove_research_item(&mut self, id: Id) {
        if self.confirm_research != Some(id) {
            self.confirm_research = Some(id);
            self.status = "Click delete again to remove this note and its files from attachments/.".into();
            return;
        }
        self.confirm_research = None;
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().forget_research_item(&self.library, id) {
            Ok(library) => {
                if self.selected_research == Some(id) {
                    self.selected_research = library.items_in_order().first().map(|item| item.id);
                }
                self.library = library;
                self.library_dirty = false;
                self.status = "Research note removed.".into();
            }
            Err(err) => self.status = format!("Could not remove research note: {err}"),
        }
    }

    pub fn attach_research_file(&mut self, item_id: Id, source: PathBuf) {
        self.confirm_research = None;
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().import_research_file(&self.library, item_id, &source) {
            Ok(library) => {
                self.library = library;
                self.library_dirty = false;
                self.status = "File copied into attachments/.".into();
            }
            Err(err) => self.status = format!("Could not attach file: {err}"),
        }
    }

    pub fn remove_research_file(&mut self, id: Id) {
        if self.confirm_research != Some(id) {
            self.confirm_research = Some(id);
            self.status = "Click remove again to delete this file from attachments/.".into();
            return;
        }
        self.confirm_research = None;
        let Some(store) = &self.store else {
            return;
        };
        match store.borrow().forget_research_file(&self.library, id) {
            Ok(library) => {
                self.library = library;
                self.library_dirty = false;
                self.status = "Attachment removed.".into();
            }
            Err(err) => self.status = format!("Could not remove attachment: {err}"),
        }
    }

    pub fn research_file_path(&self, id: Id) -> Option<PathBuf> {
        let file = self.library.files.iter().find(|file| file.id == id)?;
        let store = self.store.as_ref()?;
        let root = store.borrow().root.clone();
        let relative = PathBuf::from(&file.stored_name);
        if relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return None;
        }
        Some(root.join("attachments").join(relative))
    }

    fn edit_library(&mut self, edit: impl FnOnce(&mut ResearchLibrary) -> bool) {
        self.confirm_research = None;
        let mut library = self.library.clone();
        if edit(&mut library) {
            self.store_library(library, false);
        }
    }

    fn store_library(&mut self, library: ResearchLibrary, immediate: bool) {
        if !immediate {
            self.library = library;
            self.library_dirty = true;
            self.note_edit();
            return;
        }
        let Some(store) = &self.store else {
            self.library = library;
            self.library_dirty = false;
            return;
        };
        match store.borrow().persist_library(&library) {
            Ok(()) => {
                self.library = library;
                self.library_dirty = false;
                self.status = "Research saved locally.".into();
            }
            Err(err) => self.status = format!("Could not save research: {err}"),
        }
    }

    /// Move binder research notes into the research library, then drop those binder items.
    fn absorb_binder_research(&mut self) -> bool {
        let note_ids = self.binder.research_notes();
        let folder_ids = self.binder.research_folders();
        if note_ids.is_empty() && folder_ids.is_empty() {
            return true;
        }
        let Some(store) = self.store.clone() else {
            return true;
        };
        let mut library = self.library.clone();
        let mut changed_library = false;
        for id in &note_ids {
            let Some(node) = self.binder.get(*id).cloned() else {
                continue;
            };
            let from_document = node.document_id.and_then(|doc_id| {
                store
                    .borrow()
                    .load_document(doc_id)
                    .ok()
                    .map(|document| document.to_plain_text())
            });
            let text = match from_document {
                Some(text) if !text.trim().is_empty() => text,
                _ => node.synopsis,
            };
            if library.import_note(node.title, text) {
                changed_library = true;
            }
        }
        if changed_library {
            match store.borrow().persist_library(&library) {
                Ok(()) => {
                    self.library = library;
                    self.library_dirty = false;
                }
                Err(err) => {
                    self.status = format!("Could not move research notes: {err}");
                    return false;
                }
            }
        }
        let mut remove_ids = note_ids;
        remove_ids.extend(folder_ids);
        {
            let store = store.borrow();
            for id in &remove_ids {
                if let Some(document_id) = self.binder.get(*id).and_then(|node| node.document_id) {
                    if let Err(err) = store.delete_document(document_id) {
                        self.status = format!("Could not move research notes: {err}");
                        return false;
                    }
                }
                if let Err(err) = store.delete_node(*id) {
                    self.status = format!("Could not move research notes: {err}");
                    return false;
                }
            }
        }
        self.binder
            .nodes
            .retain(|node| !remove_ids.contains(&node.id));
        if self.selected.is_some_and(|id| remove_ids.contains(&id)) {
            self.selected = None;
            self.session = None;
            self.editor_text.clear();
            self.editor_html.clear();
        }
        self.persist_binder()
    }

    fn persist_library_now(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.borrow().persist_library(&self.library) {
            Ok(()) => true,
            Err(err) => {
                self.status = format!("Could not save research: {err}");
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
        if !self.persist_binder() {
            return;
        }
        if self.plot.detach_scenes(&nodes) && !self.persist_plot_now() {
            self.plot_dirty = true;
            self.note_edit();
            return;
        }
        if self.cast.detach_scenes(&nodes) && !self.persist_cast_now() {
            self.cast_dirty = true;
            self.note_edit();
            return;
        }
        if self.world.detach_scenes(&nodes) && !self.persist_world_now() {
            self.world_dirty = true;
            self.note_edit();
            return;
        }
        if self.library.detach_scenes(&nodes) && !self.persist_library_now() {
            self.library_dirty = true;
            self.note_edit();
            return;
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

    fn infer_parent(&self, kind: NodeKind) -> Option<Id> {
        self.binder
            .parent_for_new_item(kind, self.selected, self.series_scope())
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

pub enum CompiledFormat {
    Markdown,
    Docx,
}

pub struct ExportFile {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub filter_name: &'static str,
    pub extension: &'static str,
    pub kind_label: &'static str,
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
        state.apply_editor_html(
            concat!(
                r#"<p>The <span class="ws-comment" data-comment="which one?">harbor</span> was "#,
                r#"<mark data-highlight="yellow">quiet</mark>.</p>"#,
                r#"<p><span class="ws-comment" data-comment="cut">Too long.</span></p>"#,
            )
            .into(),
        );
        assert_eq!(state.review_items().len(), 3);
        let epoch = state.editor_epoch;

        state.set_note_kind(0, NoteKind::Question);
        state.resolve_note(0, true);
        state.set_note_text(1, "cut this");
        assert!(state.editor_epoch > epoch);
        assert!(state.editor_html.contains(r#"data-resolved="true""#));
        assert!(state.editor_html.contains(r#"data-kind="question""#));

        // What the webview sends back after the redraw is not a new edit.
        assert!(state.same_editor_html(&state.editor_html.clone()));

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
    fn tracked_changes_are_accepted_rejected_and_kept_across_reopen() {
        let (mut state, dir) = open_state("track");
        state.apply_editor_html("<p>She opened the door.</p><p>Nobody spoke.</p>".into());
        assert!(!state.is_tracking());
        assert!(state.tracked_changes().is_empty());

        state.start_tracking();
        assert!(state.is_tracking());
        assert!(state.tracked_changes().is_empty());

        state.apply_editor_html(
            "<p>She opened the red door.</p><p>A gull cried.</p><p>Nobody spoke.</p><p>The end.</p>"
                .into(),
        );
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
    fn spelling_uses_the_dictionary_project_words_and_bible_names() {
        let (mut state, dir) = open_state("spell");
        assert_eq!(state.spelling_language().as_deref(), Some("en_US"));
        state.apply_editor_html("<p>Maravance did not recieve teh letter. Teh end, said Zhul.</p>".into());
        let words: Vec<String> = state.unknown_words().into_iter().map(|item| item.word).collect();
        assert_eq!(words, ["Maravance", "recieve", "teh", "Teh", "Zhul"]);
        assert!(state.spelling_suggestions("recieve").contains(&"receive".to_string()));

        state.add_project_word("Zhul");
        state.add_project_word("zhul");
        assert_eq!(state.project_words(), ["zhul"]);
        state.add_character();
        let id = state.cast.characters.last().unwrap().id;
        state.set_character_name(id, "Maravance Oduya".into());
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
