//! Compile profiles: what a compiled book looks like, chosen once and kept
//! with the project.
//!
//! A project with no profiles of its own has the built-in ones. Changing,
//! copying, or removing one gives the project its own list, starting from
//! the built-in ones. Built-in profiles keep fixed ids, so two computers that
//! sync the same project agree on which profile is which.

use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

/// The file a profile makes. More formats come with later stages of Phase 6.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileFormat {
    #[default]
    Docx,
    Markdown,
}

impl CompileFormat {
    pub const ALL: [CompileFormat; 2] = [CompileFormat::Docx, CompileFormat::Markdown];

    pub fn label(self) -> &'static str {
        match self {
            CompileFormat::Docx => "Word (DOCX)",
            CompileFormat::Markdown => "Markdown",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            CompileFormat::Docx => "docx",
            CompileFormat::Markdown => "md",
        }
    }

    /// Whether the format has pages, so the page settings apply to it.
    pub fn has_pages(self) -> bool {
        matches!(self, CompileFormat::Docx)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSize {
    #[default]
    Letter,
    A4,
}

impl PageSize {
    pub const ALL: [PageSize; 2] = [PageSize::Letter, PageSize::A4];

    pub fn label(self) -> &'static str {
        match self {
            PageSize::Letter => "Letter",
            PageSize::A4 => "A4",
        }
    }

    /// Width and height in twentieths of a point.
    pub fn twips(self) -> (u32, u32) {
        match self {
            PageSize::Letter => (12_240, 15_840),
            PageSize::A4 => (11_906, 16_838),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineSpacing {
    #[default]
    Single,
    OneAndHalf,
    Double,
}

impl LineSpacing {
    pub const ALL: [LineSpacing; 3] = [LineSpacing::Single, LineSpacing::OneAndHalf, LineSpacing::Double];

    pub fn label(self) -> &'static str {
        match self {
            LineSpacing::Single => "Single",
            LineSpacing::OneAndHalf => "1.5 lines",
            LineSpacing::Double => "Double",
        }
    }

    /// The line height as Word counts it: 240 is one line.
    pub fn word_line(self) -> i32 {
        match self {
            LineSpacing::Single => 240,
            LineSpacing::OneAndHalf => 360,
            LineSpacing::Double => 480,
        }
    }
}

pub const FONT_SIZE_RANGE: (u32, u32) = (8, 24);
/// Margins in tenths of an inch.
pub const MARGIN_RANGE: (u32, u32) = (2, 30);
const NAME_MAX: usize = 60;
const FONT_MAX: usize = 60;
/// More than this many profiles in a project file are ignored.
pub const PROFILE_LIMIT: usize = 40;

/// The page of a format that has pages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSetup {
    pub font: String,
    /// In points.
    pub font_size: u32,
    pub line_spacing: LineSpacing,
    pub size: PageSize,
    /// Each margin, in tenths of an inch.
    pub margin: u32,
    /// Paragraphs start with an indent rather than a space between them.
    pub indent: bool,
    /// Each chapter starts on a new page.
    pub chapter_new_page: bool,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            font: "Times New Roman".into(),
            font_size: 12,
            line_spacing: LineSpacing::Single,
            size: PageSize::Letter,
            margin: 10,
            indent: true,
            chapter_new_page: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileProfile {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub format: CompileFormat,
    /// The book's title first, on a page of its own.
    #[serde(default = "default_true")]
    pub title_page: bool,
    #[serde(default)]
    pub page: PageSetup,
}

fn default_true() -> bool {
    true
}

const MANUSCRIPT_ID: &str = "6a0c1f3e-2b7d-4c4e-9a51-0d8e6b1f7c01";
const PLAIN_TEXT_ID: &str = "6a0c1f3e-2b7d-4c4e-9a51-0d8e6b1f7c02";

fn fixed_id(text: &str) -> Id {
    text.parse().expect("built-in profile ids are valid")
}

/// The profiles every project starts with.
pub fn built_in_profiles() -> Vec<CompileProfile> {
    vec![
        CompileProfile {
            id: fixed_id(MANUSCRIPT_ID),
            name: "Manuscript".into(),
            format: CompileFormat::Docx,
            title_page: true,
            // Standard manuscript format: a 12 point serif, double spaced,
            // one-inch margins, indented paragraphs.
            page: PageSetup { line_spacing: LineSpacing::Double, ..PageSetup::default() },
        },
        CompileProfile {
            id: fixed_id(PLAIN_TEXT_ID),
            name: "Plain text".into(),
            format: CompileFormat::Markdown,
            title_page: true,
            page: PageSetup::default(),
        },
    ]
}

/// The built-in profile with this id, as it ships.
pub fn built_in_profile(id: Id) -> Option<CompileProfile> {
    built_in_profiles().into_iter().find(|profile| profile.id == id)
}

impl CompileProfile {
    /// The profile with every value inside what the program accepts. A
    /// project file can come from another computer, so nothing in it is
    /// taken as it is.
    pub fn normalized(mut self) -> Self {
        self.name = clean_text(&self.name, NAME_MAX);
        if self.name.is_empty() {
            self.name = "Untitled profile".into();
        }
        self.page.font = clean_text(&self.page.font, FONT_MAX);
        if self.page.font.is_empty() {
            self.page.font = PageSetup::default().font;
        }
        self.page.font_size = self.page.font_size.clamp(FONT_SIZE_RANGE.0, FONT_SIZE_RANGE.1);
        self.page.margin = self.page.margin.clamp(MARGIN_RANGE.0, MARGIN_RANGE.1);
        self
    }

    pub fn is_built_in(&self) -> bool {
        built_in_profile(self.id).is_some()
    }
}

fn clean_text(text: &str, max: usize) -> String {
    text.chars().filter(|ch| !ch.is_control()).collect::<String>().trim().chars().take(max).collect()
}

/// The profile chosen last for a book on this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookProfile {
    pub book_id: Id,
    pub profile_id: Id,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_profiles_keep_their_ids_and_values_are_kept_in_range() {
        let first = built_in_profiles();
        assert_eq!(first, built_in_profiles(), "the same ids on every computer");
        assert!(first.iter().all(CompileProfile::is_built_in));
        let manuscript = &first[0];
        assert_eq!(manuscript.page.line_spacing, LineSpacing::Double);
        assert_eq!(manuscript.page.font_size, 12);

        let wild = CompileProfile {
            id: Id::new(),
            name: "  \u{7}Bad\nname  ".into(),
            format: CompileFormat::Docx,
            title_page: false,
            page: PageSetup { font: String::new(), font_size: 400, margin: 0, ..PageSetup::default() },
        }
        .normalized();
        assert_eq!(wild.name, "Badname");
        assert_eq!(wild.page.font, "Times New Roman");
        assert_eq!(wild.page.font_size, FONT_SIZE_RANGE.1);
        assert_eq!(wild.page.margin, MARGIN_RANGE.0);
        assert!(!wild.is_built_in());
    }

    #[test]
    fn a_profile_from_a_file_with_only_a_name_and_id_has_defaults() {
        let json = r#"{"id":"6a0c1f3e-2b7d-4c4e-9a51-0d8e6b1f7c99","name":"Mine"}"#;
        let profile: CompileProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.format, CompileFormat::Docx);
        assert!(profile.title_page);
        assert_eq!(profile.page, PageSetup::default());
    }
}
