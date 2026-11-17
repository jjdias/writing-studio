use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

use crate::WritingLog;

pub const SIDEBAR_WIDTH_DEFAULT: u32 = 260;
pub const SIDEBAR_WIDTH_MIN: u32 = 180;
pub const SIDEBAR_WIDTH_MAX: u32 = 480;
pub const INSPECTOR_WIDTH_DEFAULT: u32 = 300;
pub const INSPECTOR_WIDTH_MIN: u32 = 220;
pub const INSPECTOR_WIDTH_MAX: u32 = 560;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarSide {
    Left,
    Right,
}

impl Default for SidebarSide {
    fn default() -> Self {
        Self::Left
    }
}

/// Which side the binder sits on, and whether each side panel is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelChrome {
    pub sidebar_side: SidebarSide,
    pub sidebar_visible: bool,
    pub inspector_visible: bool,
}

impl Default for PanelChrome {
    fn default() -> Self {
        PanelLayout::default().chrome()
    }
}

/// Pixel widths for the binder and the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelWidths {
    pub sidebar: u32,
    pub inspector: u32,
}

impl Default for PanelWidths {
    fn default() -> Self {
        PanelLayout::default().widths()
    }
}

/// Workbench arrangement stored with the project. Missing on older projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelLayout {
    #[serde(default)]
    pub sidebar_side: SidebarSide,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default = "default_inspector_width")]
    pub inspector_width: u32,
    #[serde(default = "default_visible")]
    pub sidebar_visible: bool,
    #[serde(default = "default_visible")]
    pub inspector_visible: bool,
}

fn default_sidebar_width() -> u32 {
    SIDEBAR_WIDTH_DEFAULT
}

fn default_inspector_width() -> u32 {
    INSPECTOR_WIDTH_DEFAULT
}

fn default_visible() -> bool {
    true
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            sidebar_side: SidebarSide::Left,
            sidebar_width: SIDEBAR_WIDTH_DEFAULT,
            inspector_width: INSPECTOR_WIDTH_DEFAULT,
            sidebar_visible: true,
            inspector_visible: true,
        }
    }
}

impl PanelLayout {
    pub fn chrome(self) -> PanelChrome {
        PanelChrome {
            sidebar_side: self.sidebar_side,
            sidebar_visible: self.sidebar_visible,
            inspector_visible: self.inspector_visible,
        }
    }

    pub fn widths(self) -> PanelWidths {
        PanelWidths {
            sidebar: self.sidebar_width,
            inspector: self.inspector_width,
        }
    }

    pub fn with_chrome(mut self, chrome: PanelChrome) -> Self {
        self.sidebar_side = chrome.sidebar_side;
        self.sidebar_visible = chrome.sidebar_visible;
        self.inspector_visible = chrome.inspector_visible;
        self
    }

    pub fn with_widths(mut self, widths: PanelWidths) -> Self {
        self.sidebar_width = widths.sidebar;
        self.inspector_width = widths.inspector;
        self.clamp()
    }

    pub fn clamp(mut self) -> Self {
        self.sidebar_width = self
            .sidebar_width
            .clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        self.inspector_width = self
            .inspector_width
            .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
        self
    }
}

/// Workbench color theme stored with the project. Missing on older projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    Paper,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Paper => "paper",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Paper => "Paper",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "paper" => Some(Self::Paper),
            _ => None,
        }
    }
}

/// The typeface the manuscript is set in. Missing on older projects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProseFont {
    #[default]
    Serif,
    Sans,
}

impl ProseFont {
    pub const ALL: [ProseFont; 2] = [ProseFont::Serif, ProseFont::Sans];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Serif => "serif",
            Self::Sans => "sans",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Serif => "Serif",
            Self::Sans => "Sans serif",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "serif" => Some(Self::Serif),
            "sans" => Some(Self::Sans),
            _ => None,
        }
    }
}

/// How the page is set while writing: line length, line height, space
/// between paragraphs, and first-line indent. It does not change compile.
/// Each value is kept within the limits below, and a missing one is
/// today's look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageStyle {
    /// Characters to a line.
    #[serde(default = "default_line_length")]
    pub line_length: u32,
    /// Line height in hundredths: 165 is 1.65 lines.
    #[serde(default = "default_line_height")]
    pub line_height: u32,
    /// Space after a paragraph, in tenths of a line: 4 is 0.4 lines.
    #[serde(default = "default_paragraph_spacing")]
    pub paragraph_spacing: u32,
    /// First-line indent, in tenths of an em: 15 is 1.5 em.
    #[serde(default)]
    pub first_line_indent: u32,
}

pub const LINE_LENGTH_RANGE: (u32, u32) = (45, 100);
pub const LINE_HEIGHT_RANGE: (u32, u32) = (120, 220);
pub const PARAGRAPH_SPACING_RANGE: (u32, u32) = (0, 20);
pub const FIRST_LINE_INDENT_RANGE: (u32, u32) = (0, 40);

fn default_line_length() -> u32 {
    70
}

fn default_line_height() -> u32 {
    165
}

fn default_paragraph_spacing() -> u32 {
    // About the 12 pixels paragraphs had before this setting, at 17 px text.
    4
}

impl Default for PageStyle {
    fn default() -> Self {
        Self {
            line_length: default_line_length(),
            line_height: default_line_height(),
            paragraph_spacing: default_paragraph_spacing(),
            first_line_indent: 0,
        }
    }
}

impl PageStyle {
    /// The same style with every value inside its limits.
    pub fn clamp(self) -> Self {
        let within = |value: u32, (low, high): (u32, u32)| value.clamp(low, high);
        Self {
            line_length: within(self.line_length, LINE_LENGTH_RANGE),
            line_height: within(self.line_height, LINE_HEIGHT_RANGE),
            paragraph_spacing: within(self.paragraph_spacing, PARAGRAPH_SPACING_RANGE),
            first_line_indent: within(self.first_line_indent, FIRST_LINE_INDENT_RANGE),
        }
    }
}

/// Width after a horizontal drag. `positive` means moving the pointer right grows the panel.
/// Word target for one book in a series project. Missing books use the project goal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookWordGoal {
    pub book_id: Id,
    pub goal: u32,
}

pub fn drag_width(origin: u32, delta_px: i32, positive: bool, min: u32, max: u32) -> u32 {
    let next = if positive {
        origin as i32 + delta_px
    } else {
        origin as i32 - delta_px
    };
    next.clamp(min as i32, max as i32) as u32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub daily_word_goal: u32,
    pub project_word_goal: u32,
    pub compile_title_page: bool,
    pub autosave_seconds: u32,
    /// Daily baseline and completed writing sessions. Missing on older projects.
    #[serde(default)]
    pub writing: WritingLog,
    /// Binder side, panel widths, and which panels are open. Missing on older projects.
    #[serde(default)]
    pub layout: PanelLayout,
    /// Dark, light, or paper. Missing on older projects.
    #[serde(default)]
    pub theme: Theme,
    /// Serif or sans for the manuscript. Missing on older projects, which use serif.
    #[serde(default)]
    pub prose_font: ProseFont,
    /// Line length, line height, paragraph spacing, and indent for the page.
    /// Missing on older projects, which keep today's look.
    #[serde(default)]
    pub page: PageStyle,
    /// Binder items folded shut in the library and the scene list. Missing
    /// on older projects, which open with everything showing.
    #[serde(default)]
    pub collapsed: Vec<Id>,
    /// Book whose compile, word count, and word target are shown. Missing means the first book.
    #[serde(default)]
    pub compile_book_id: Option<Id>,
    /// Per-book word targets. A book with no entry uses `project_word_goal` when it is the first book.
    #[serde(default)]
    pub book_word_goals: Vec<BookWordGoal>,
    /// Binder, corkboard, outliner, and search show every book. Compile still uses `compile_book_id`.
    /// Missing on older projects, which open on the whole series.
    #[serde(default = "default_show_whole_series")]
    pub show_whole_series: bool,
    /// Spellcheck switch, dictionary, and the project's own words. Missing on older projects.
    #[serde(default)]
    pub spelling: SpellingSettings,
}

/// Spellcheck choices stored with the project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellingSettings {
    #[serde(default = "default_visible")]
    pub enabled: bool,
    /// Dictionary name, such as `en_US`. A name that cannot be found falls back to the bundled one.
    #[serde(default = "default_spelling_language")]
    pub language: String,
    /// Words this project accepts: names, places, invented terms. Lowercase, sorted.
    #[serde(default)]
    pub words: Vec<String>,
}

fn default_spelling_language() -> String {
    "en_US".into()
}

impl Default for SpellingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            language: default_spelling_language(),
            words: Vec::new(),
        }
    }
}

fn default_show_whole_series() -> bool {
    true
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            daily_word_goal: 500,
            project_word_goal: 80_000,
            compile_title_page: true,
            autosave_seconds: 2,
            writing: WritingLog::default(),
            layout: PanelLayout::default(),
            theme: Theme::Light,
            prose_font: ProseFont::Serif,
            page: PageStyle::default(),
            collapsed: Vec::new(),
            compile_book_id: None,
            book_word_goals: Vec::new(),
            show_whole_series: true,
            spelling: SpellingSettings::default(),
        }
    }
}

impl ProjectSettings {
    /// Target for `book_id`. The first book keeps the legacy project goal until it has its own entry.
    pub fn goal_for_book(&self, book_id: Id, first_book: bool) -> u32 {
        if let Some(entry) = self
            .book_word_goals
            .iter()
            .find(|entry| entry.book_id == book_id)
        {
            return entry.goal;
        }
        if first_book {
            self.project_word_goal
        } else {
            80_000
        }
    }

    pub fn set_goal_for_book(&mut self, book_id: Id, goal: u32, first_book: bool) {
        if let Some(entry) = self
            .book_word_goals
            .iter_mut()
            .find(|entry| entry.book_id == book_id)
        {
            entry.goal = goal;
        } else {
            self.book_word_goals.push(BookWordGoal { book_id, goal });
        }
        if first_book {
            self.project_word_goal = goal;
        }
    }

    /// Remove a permanently deleted book's target. When it was the compile book, store `next_book`.
    pub fn drop_book(&mut self, book_id: Id, next_book: Option<Id>) -> bool {
        let before = self.book_word_goals.len();
        self.book_word_goals.retain(|entry| entry.book_id != book_id);
        let mut changed = self.book_word_goals.len() != before;
        if self.compile_book_id == Some(book_id) {
            self.compile_book_id = next_book;
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_keep_the_default_layout() {
        let json = r#"{"daily_word_goal":250,"project_word_goal":1000,"compile_title_page":false,"autosave_seconds":4}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.layout, PanelLayout::default());
    }

    #[test]
    fn partial_layout_fills_the_other_fields() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"layout":{"sidebar_side":"right","sidebar_width":900}}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        let layout = settings.layout.clamp();
        assert_eq!(layout.sidebar_side, SidebarSide::Right);
        assert_eq!(layout.sidebar_width, SIDEBAR_WIDTH_MAX);
        assert_eq!(layout.inspector_width, INSPECTOR_WIDTH_DEFAULT);
        assert!(layout.sidebar_visible);
        assert!(layout.inspector_visible);
    }

    #[test]
    fn theme_defaults_to_light_and_roundtrips() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.theme, Theme::Light);
        let saved = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"theme":"paper"}"#;
        let settings: ProjectSettings = serde_json::from_str(saved).unwrap();
        assert_eq!(settings.theme, Theme::Paper);
        assert_eq!(Theme::from_id("light"), Some(Theme::Light));
        assert_eq!(Theme::from_id("sepia"), None);
        assert_eq!(settings.prose_font, ProseFont::Serif);
        let sans = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"prose_font":"sans"}"#;
        let settings: ProjectSettings = serde_json::from_str(sans).unwrap();
        assert_eq!(settings.prose_font, ProseFont::Sans);
        assert_eq!(ProseFont::from_id("mono"), None);
        assert!(settings.compile_book_id.is_none());
        assert!(settings.book_word_goals.is_empty());
        assert!(settings.show_whole_series);
    }

    #[test]
    fn the_series_view_is_remembered() {
        let mut settings = ProjectSettings::default();
        assert!(settings.show_whole_series);
        settings.show_whole_series = false;
        settings.compile_book_id = Some(Id::new());
        let saved = serde_json::to_string(&settings).unwrap();
        let loaded: ProjectSettings = serde_json::from_str(&saved).unwrap();
        assert!(!loaded.show_whole_series);
        assert_eq!(loaded.compile_book_id, settings.compile_book_id);
    }

    #[test]
    fn each_book_can_keep_its_own_word_goal() {
        let mut settings = ProjectSettings::default();
        settings.project_word_goal = 50_000;
        let first = Id::new();
        let second = Id::new();
        assert_eq!(settings.goal_for_book(first, true), 50_000);
        assert_eq!(settings.goal_for_book(second, false), 80_000);
        settings.set_goal_for_book(second, 30_000, false);
        settings.set_goal_for_book(first, 40_000, true);
        assert_eq!(settings.goal_for_book(first, true), 40_000);
        assert_eq!(settings.goal_for_book(second, false), 30_000);
        assert_eq!(settings.project_word_goal, 40_000);
        let saved = serde_json::to_string(&settings).unwrap();
        let loaded: ProjectSettings = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.goal_for_book(second, false), 30_000);
        let third = Id::new();
        settings.compile_book_id = Some(second);
        assert!(settings.drop_book(second, Some(third)));
        assert!(settings.book_word_goals.iter().all(|entry| entry.book_id != second));
        assert_eq!(settings.compile_book_id, Some(third));
        assert_eq!(settings.goal_for_book(first, true), 40_000);
        assert!(!settings.drop_book(Id::new(), Some(first)));
    }

    #[test]
    fn drag_width_follows_the_pointer_and_stays_in_range() {
        assert_eq!(drag_width(260, 40, true, 180, 480), 300);
        assert_eq!(drag_width(260, 40, false, 180, 480), 220);
        assert_eq!(drag_width(260, 500, true, 180, 480), 480);
        assert_eq!(drag_width(260, 500, false, 180, 480), 180);
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;

    #[test]
    fn the_page_style_keeps_todays_look_when_missing_and_stays_in_its_limits() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.page, PageStyle::default());
        // A file that sets only some values keeps the defaults for the rest.
        let some = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"page":{"first_line_indent":15}}"#;
        let settings: ProjectSettings = serde_json::from_str(some).unwrap();
        assert_eq!(settings.page.first_line_indent, 15);
        assert_eq!(settings.page.line_length, 70);
        let wild = PageStyle { line_length: 500, line_height: 10, paragraph_spacing: 99, first_line_indent: 99 }.clamp();
        assert_eq!(wild, PageStyle { line_length: 100, line_height: 120, paragraph_spacing: 20, first_line_indent: 40 });
    }
}
