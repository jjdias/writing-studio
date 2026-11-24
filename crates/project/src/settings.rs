use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

use crate::compile::{built_in_profile, built_in_profiles, clean_text, BookProfile, CompileProfile, PROFILE_LIMIT};
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

/// Where the line being typed stays on screen, or `Off` to scroll as usual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixedScroll {
    #[default]
    Off,
    Top,
    Middle,
    Bottom,
}

/// What stays at full strength while the rest of the page fades.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fade {
    #[default]
    Off,
    Paragraph,
    Sentence,
}

/// How the Markdown page shows and takes text. Missing on older projects,
/// which get live preview and nothing changed as it is typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownOptions {
    /// Every Markdown mark shown as typed, not only where the caret is.
    #[serde(default)]
    pub source_mode: bool,
    /// Curly quotes, dashes, and an ellipsis as they are typed.
    #[serde(default)]
    pub smart_typography: bool,
    /// A typed `*`, `_`, `"`, or bracket gets its closing pair.
    #[serde(default)]
    pub pair_marks: bool,
}

/// Helps for keeping one's place on the page while writing. Fixed
/// scrolling, the marker, fading, and hiding everything but the page while
/// one types are off until chosen. Missing on older projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeepPlace {
    #[serde(default)]
    pub scroll: FixedScroll,
    /// A faint band under the line with the caret.
    #[serde(default)]
    pub line_marker: bool,
    #[serde(default)]
    pub fade: Fade,
    /// Everything but the page (the bars, the binder, and Context) fades
    /// while one types, and comes back when the pointer moves. Under a new
    /// key: `quiet_bars`, written when this was on by default, is not read.
    #[serde(default)]
    pub hide_while_typing: bool,
}

impl Default for KeepPlace {
    fn default() -> Self {
        Self {
            scroll: FixedScroll::Off,
            line_marker: false,
            fade: Fade::Off,
            hide_while_typing: false,
        }
    }
}

impl FixedScroll {
    pub const ALL: [FixedScroll; 4] = [Self::Off, Self::Top, Self::Middle, Self::Bottom];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Scroll as usual",
            Self::Top => "Near the top",
            Self::Middle => "In the middle",
            Self::Bottom => "Near the bottom",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|item| item.as_str() == value)
    }
}

impl Fade {
    pub const ALL: [Fade; 3] = [Self::Off, Self::Paragraph, Self::Sentence];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Paragraph => "paragraph",
            Self::Sentence => "sentence",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Nothing fades",
            Self::Paragraph => "All but this paragraph",
            Self::Sentence => "All but this sentence",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|item| item.as_str() == value)
    }
}

/// Width after a horizontal drag. `positive` means moving the pointer right grows the panel.
/// Word target for one book in a series project. Missing books use the project goal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookWordGoal {
    pub book_id: Id,
    pub goal: u32,
    /// The day to reach the target by, as `YYYY-MM-DD`. Missing means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
}

/// A book's subtitle, printed on its title page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSubtitle {
    pub book_id: Id,
    pub subtitle: String,
}

const AUTHOR_MAX: usize = 120;
const SUBTITLE_MAX: usize = 200;

/// Where a book stands against its deadline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// The target is reached.
    Reached,
    /// `days` left, today included, and the words a day that meet it.
    Due { days: u32, per_day: u32 },
    /// The deadline has passed with `left` words still to write.
    Passed { days_late: u32, left: u32 },
}

/// The pace for `words` written toward `goal` by `deadline`, seen on
/// `today` (both `YYYY-MM-DD`). Today counts as a day to write in, so on the
/// deadline itself all that is left is due today. `None` when a day is not
/// a date or there is no target.
pub fn pace(goal: u32, words: u32, today: &str, deadline: &str) -> Option<Pace> {
    if goal == 0 {
        return None;
    }
    let until = writing_studio_core::days_between(today, deadline)?;
    let left = goal.saturating_sub(words);
    if left == 0 {
        return Some(Pace::Reached);
    }
    if until < 0 {
        return Some(Pace::Passed { days_late: (-until) as u32, left });
    }
    let days = until as u32 + 1;
    Some(Pace::Due { days, per_day: left.div_ceil(days) })
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
    pub autosave_seconds: u32,
    /// The project's own compile profiles. Empty means the built-in ones.
    #[serde(default)]
    pub compile_profiles: Vec<CompileProfile>,
    /// The profile chosen last for each book, on this computer.
    #[serde(default)]
    pub compile_choices: Vec<BookProfile>,
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
    /// Fixed scrolling, the line marker, and fading. Missing on older
    /// projects, which have none of them on.
    #[serde(default)]
    pub keep_place: KeepPlace,
    /// Source mode, smart typography, and pairing on the Markdown page.
    #[serde(default)]
    pub markdown: MarkdownOptions,
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
    /// Binder, mind map, outliner, and search show every book. Compile still uses `compile_book_id`.
    /// Missing on older projects, which open on the whole series.
    #[serde(default = "default_show_whole_series")]
    pub show_whole_series: bool,
    /// Spellcheck switch, dictionary, and the project's own words. Missing on older projects.
    #[serde(default)]
    pub spelling: SpellingSettings,
    /// The writer's name as the books print it. Shared, like the title.
    #[serde(default)]
    pub author: String,
    /// Each book's subtitle. A book with no entry has none.
    #[serde(default)]
    pub book_subtitles: Vec<BookSubtitle>,
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
            autosave_seconds: 2,
            compile_profiles: Vec::new(),
            compile_choices: Vec::new(),
            writing: WritingLog::default(),
            layout: PanelLayout::default(),
            theme: Theme::Light,
            prose_font: ProseFont::Serif,
            page: PageStyle::default(),
            keep_place: KeepPlace::default(),
            markdown: MarkdownOptions::default(),
            collapsed: Vec::new(),
            compile_book_id: None,
            book_word_goals: Vec::new(),
            show_whole_series: true,
            spelling: SpellingSettings::default(),
            author: String::new(),
            book_subtitles: Vec::new(),
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
            self.book_word_goals.push(BookWordGoal { book_id, goal, deadline: None });
        }
        if first_book {
            self.project_word_goal = goal;
        }
    }

    pub fn deadline_for_book(&self, book_id: Id) -> Option<&str> {
        self.book_word_goals
            .iter()
            .find(|entry| entry.book_id == book_id)
            .and_then(|entry| entry.deadline.as_deref())
    }

    /// Set or clear a book's deadline. A book without its own target entry
    /// gets one holding the target it has now, so the target does not move.
    pub fn set_deadline_for_book(&mut self, book_id: Id, deadline: Option<String>, first_book: bool) {
        let goal = self.goal_for_book(book_id, first_book);
        match self
            .book_word_goals
            .iter_mut()
            .find(|entry| entry.book_id == book_id)
        {
            Some(entry) => entry.deadline = deadline,
            None => self.book_word_goals.push(BookWordGoal { book_id, goal, deadline }),
        }
    }

    /// Remove a permanently deleted book's target. When it was the compile book, store `next_book`.
    pub fn drop_book(&mut self, book_id: Id, next_book: Option<Id>) -> bool {
        let before = self.book_word_goals.len();
        self.book_word_goals.retain(|entry| entry.book_id != book_id);
        let subtitles = self.book_subtitles.len();
        self.book_subtitles.retain(|entry| entry.book_id != book_id);
        let mut changed = self.book_word_goals.len() != before || self.book_subtitles.len() != subtitles;
        if self.compile_book_id == Some(book_id) {
            self.compile_book_id = next_book;
            changed = true;
        }
        changed
    }

    /// The author's name, cleaned: a project file can come from anywhere.
    pub fn author_name(&self) -> String {
        clean_text(&self.author, AUTHOR_MAX)
    }

    pub fn set_author(&mut self, author: &str) {
        self.author = clean_text(author, AUTHOR_MAX);
    }

    pub fn subtitle_for_book(&self, book_id: Id) -> String {
        self.book_subtitles
            .iter()
            .find(|entry| entry.book_id == book_id)
            .map(|entry| clean_text(&entry.subtitle, SUBTITLE_MAX))
            .unwrap_or_default()
    }

    /// Set a book's subtitle; an empty one removes its entry.
    pub fn set_subtitle_for_book(&mut self, book_id: Id, subtitle: &str) {
        let subtitle = clean_text(subtitle, SUBTITLE_MAX);
        self.book_subtitles.retain(|entry| entry.book_id != book_id);
        if !subtitle.is_empty() {
            self.book_subtitles.push(BookSubtitle { book_id, subtitle });
        }
    }

    /// The compile profiles, each with its values in range: the project's
    /// own, or the built-in ones when it has none.
    pub fn profiles(&self) -> Vec<CompileProfile> {
        if self.compile_profiles.is_empty() {
            return built_in_profiles();
        }
        let mut seen = Vec::new();
        self.compile_profiles
            .iter()
            .filter(|profile| {
                let first = !seen.contains(&profile.id);
                seen.push(profile.id);
                first
            })
            .take(PROFILE_LIMIT)
            .cloned()
            .map(CompileProfile::normalized)
            .collect()
    }

    pub fn profile(&self, id: Id) -> Option<CompileProfile> {
        self.profiles().into_iter().find(|profile| profile.id == id)
    }

    /// Keep a changed profile, or add a new one at the end.
    pub fn save_profile(&mut self, profile: CompileProfile) {
        let mut profiles = self.profiles();
        let profile = profile.normalized();
        match profiles.iter().position(|kept| kept.id == profile.id) {
            Some(at) => profiles[at] = profile,
            None if profiles.len() < PROFILE_LIMIT => profiles.push(profile),
            None => return,
        }
        self.compile_profiles = profiles;
    }

    /// A copy of a profile under a new id, right after it. Returns its id.
    pub fn copy_profile(&mut self, id: Id) -> Option<Id> {
        let mut profiles = self.profiles();
        if profiles.len() >= PROFILE_LIMIT {
            return None;
        }
        let at = profiles.iter().position(|profile| profile.id == id)?;
        let mut copy = profiles[at].clone();
        copy.id = Id::new();
        copy.name = format!("{} copy", copy.name);
        let copy = copy.normalized();
        let new_id = copy.id;
        profiles.insert(at + 1, copy);
        self.compile_profiles = profiles;
        Some(new_id)
    }

    /// Remove a profile. Removing the last one brings back the built-in ones.
    pub fn remove_profile(&mut self, id: Id) {
        let mut profiles = self.profiles();
        profiles.retain(|profile| profile.id != id);
        self.compile_profiles = profiles;
        self.compile_choices.retain(|choice| choice.profile_id != id);
    }

    /// Put a built-in profile back as it ships, where it is, or at the end
    /// when it was removed.
    pub fn restore_profile(&mut self, id: Id) {
        let Some(original) = built_in_profile(id) else {
            return;
        };
        self.save_profile(original);
    }

    /// The profile last used for `book_id` here, or the first one.
    pub fn profile_for_book(&self, book_id: Id) -> Option<CompileProfile> {
        let profiles = self.profiles();
        let chosen = self
            .compile_choices
            .iter()
            .find(|choice| choice.book_id == book_id)
            .and_then(|choice| profiles.iter().find(|profile| profile.id == choice.profile_id));
        chosen.or(profiles.first()).cloned()
    }

    pub fn choose_profile_for_book(&mut self, book_id: Id, profile_id: Id) {
        match self.compile_choices.iter_mut().find(|choice| choice.book_id == book_id) {
            Some(choice) => choice.profile_id = profile_id,
            None => self.compile_choices.push(BookProfile { book_id, profile_id }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_start_built_in_and_can_be_copied_changed_removed_and_put_back() {
        let mut settings = ProjectSettings::default();
        let built_in = built_in_profiles();
        assert_eq!(settings.profiles(), built_in);
        let manuscript = built_in[0].id;

        let copy = settings.copy_profile(manuscript).unwrap();
        let profiles = settings.profiles();
        assert_eq!(profiles.len(), built_in.len() + 1);
        assert_eq!(profiles[1].id, copy, "the copy follows what it was copied from");
        assert_eq!(profiles[1].name, "Manuscript copy");

        let mut changed = settings.profile(manuscript).unwrap();
        changed.name = "Agent submission".into();
        changed.page.font_size = 99;
        settings.save_profile(changed);
        assert_eq!(settings.profile(manuscript).unwrap().name, "Agent submission");
        assert_eq!(settings.profile(manuscript).unwrap().page.font_size, crate::FONT_SIZE_RANGE.1);

        let book = Id::new();
        settings.choose_profile_for_book(book, copy);
        assert_eq!(settings.profile_for_book(book).unwrap().id, copy);
        settings.remove_profile(copy);
        assert_eq!(settings.profile_for_book(book).unwrap().id, manuscript, "back to the first");

        settings.restore_profile(manuscript);
        assert_eq!(settings.profile(manuscript).unwrap(), built_in[0]);
        settings.remove_profile(manuscript);
        settings.restore_profile(manuscript);
        assert_eq!(settings.profiles().last().unwrap().id, manuscript, "a removed one comes back at the end");

        for profile in settings.profiles() {
            settings.remove_profile(profile.id);
        }
        assert_eq!(settings.profiles(), built_in, "with none left, the built-in ones are back");

        // Profiles are shared: they round-trip through the project file.
        settings.copy_profile(manuscript);
        let json = serde_json::to_string(&settings).unwrap();
        let read: ProjectSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(read.profiles(), settings.profiles());
    }

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

#[cfg(test)]
mod keep_place_tests {
    use super::*;

    #[test]
    fn keeping_the_place_is_off_until_chosen_and_reads_back() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.keep_place, KeepPlace::default());
        assert_eq!(settings.keep_place.scroll, FixedScroll::Off);
        let chosen = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"keep_place":{"scroll":"middle","fade":"sentence"}}"#;
        let settings: ProjectSettings = serde_json::from_str(chosen).unwrap();
        assert_eq!(settings.keep_place.scroll, FixedScroll::Middle);
        assert_eq!(settings.keep_place.fade, Fade::Sentence);
        assert!(!settings.keep_place.line_marker);
        assert!(!settings.keep_place.hide_while_typing, "nothing hides unless chosen");
        let older = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"keep_place":{"quiet_bars":true}}"#;
        let settings: ProjectSettings = serde_json::from_str(older).unwrap();
        assert!(!settings.keep_place.hide_while_typing, "the old default on is not taken");
        assert_eq!(FixedScroll::from_id("bottom"), Some(FixedScroll::Bottom));
        assert_eq!(Fade::from_id("word"), None);
    }
}

#[cfg(test)]
mod markdown_options_tests {
    use super::*;

    #[test]
    fn the_markdown_page_choices_are_off_until_chosen_and_read_back() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.markdown, MarkdownOptions::default());
        let chosen = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"markdown":{"source_mode":true,"pair_marks":true}}"#;
        let settings: ProjectSettings = serde_json::from_str(chosen).unwrap();
        assert!(settings.markdown.source_mode && settings.markdown.pair_marks);
        assert!(!settings.markdown.smart_typography);
        let back: ProjectSettings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(back.markdown, settings.markdown);
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;

    #[test]
    fn the_pace_is_the_words_a_day_left_counting_today() {
        assert_eq!(pace(1_000, 400, "2026-11-17", "2026-11-19"), Some(Pace::Due { days: 3, per_day: 200 }));
        // On the deadline itself, the rest is due today.
        assert_eq!(pace(1_000, 400, "2026-11-19", "2026-11-19"), Some(Pace::Due { days: 1, per_day: 600 }));
        // Rounded up, so keeping the pace reaches the target.
        assert_eq!(pace(1_000, 0, "2026-11-17", "2026-11-19"), Some(Pace::Due { days: 3, per_day: 334 }));
        assert_eq!(pace(1_000, 1_200, "2026-11-17", "2026-11-19"), Some(Pace::Reached));
        assert_eq!(pace(1_000, 400, "2026-11-21", "2026-11-19"), Some(Pace::Passed { days_late: 2, left: 600 }));
        assert_eq!(pace(0, 0, "2026-11-17", "2026-11-19"), None);
        assert_eq!(pace(1_000, 0, "2026-11-17", "soon"), None);
    }

    #[test]
    fn a_deadline_keeps_the_target_and_reads_back_from_older_files() {
        let mut settings = ProjectSettings::default();
        let book = Id::new();
        settings.set_deadline_for_book(book, Some("2027-01-31".into()), true);
        assert_eq!(settings.deadline_for_book(book), Some("2027-01-31"));
        assert_eq!(settings.goal_for_book(book, true), settings.project_word_goal);
        settings.set_deadline_for_book(book, None, true);
        assert_eq!(settings.deadline_for_book(book), None);
        // An entry written before deadlines existed still reads.
        let old: BookWordGoal = serde_json::from_str(&format!(r#"{{"book_id":"{book}","goal":5}}"#)).unwrap();
        assert_eq!(old.deadline, None);
        assert!(!serde_json::to_string(&old).unwrap().contains("deadline"));
    }
}
