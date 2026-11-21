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

/// How a chapter's or a part's heading reads in the compiled book.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadingStyle {
    /// "The Harbor".
    Title,
    /// "Chapter 1".
    Number,
    /// "Chapter One".
    NumberWords,
    /// "Chapter 1: The Harbor".
    #[default]
    NumberAndTitle,
}

impl HeadingStyle {
    pub const ALL: [HeadingStyle; 4] =
        [HeadingStyle::NumberAndTitle, HeadingStyle::Number, HeadingStyle::NumberWords, HeadingStyle::Title];

    pub fn label(self) -> &'static str {
        match self {
            HeadingStyle::Title => "Its title",
            HeadingStyle::Number => "Its number",
            HeadingStyle::NumberWords => "Its number in words",
            HeadingStyle::NumberAndTitle => "Its number and title",
        }
    }

    /// The heading of the `number`th one, counted from 1. A title that is
    /// only the word and a number, as a new chapter's is, is not repeated:
    /// "Chapter 2" reads "Chapter 2", not "Chapter 2: Chapter 2". A heading
    /// with no title to show falls back to the number.
    pub fn heading(self, language: BookLanguage, word: &str, number: usize, title: &str) -> String {
        let title = title.trim();
        let numbered = |count: String| if word.is_empty() { count } else { format!("{word} {count}") };
        let placeholder = title.is_empty()
            || (!word.is_empty()
                && title.to_lowercase().strip_prefix(&word.to_lowercase()).is_some_and(|rest| {
                    rest.trim().chars().all(|ch| ch.is_ascii_digit()) && !rest.trim().is_empty()
                }));
        match self {
            HeadingStyle::Title if !placeholder => title.to_string(),
            HeadingStyle::NumberWords => numbered(language.number_words(number)),
            HeadingStyle::NumberAndTitle if !placeholder => format!("{}: {title}", numbered(number.to_string())),
            _ => numbered(number.to_string()),
        }
    }
}

/// The language a book is printed in: it sets the number words, the words
/// before a number, and the headings of the matter. Each of those can still
/// be changed in the profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookLanguage {
    #[default]
    English,
    Portuguese,
    Spanish,
    French,
}

impl BookLanguage {
    pub const ALL: [BookLanguage; 4] =
        [BookLanguage::English, BookLanguage::Portuguese, BookLanguage::Spanish, BookLanguage::French];

    /// The language's name in itself, as the menu lists it.
    pub fn label(self) -> &'static str {
        match self {
            BookLanguage::English => "English",
            BookLanguage::Portuguese => "Português",
            BookLanguage::Spanish => "Español",
            BookLanguage::French => "Français",
        }
    }

    pub fn chapter_word(self) -> &'static str {
        match self {
            BookLanguage::English => "Chapter",
            BookLanguage::Portuguese | BookLanguage::Spanish => "Capítulo",
            BookLanguage::French => "Chapitre",
        }
    }

    pub fn part_word(self) -> &'static str {
        match self {
            BookLanguage::English => "Part",
            BookLanguage::Portuguese | BookLanguage::Spanish => "Parte",
            BookLanguage::French => "Partie",
        }
    }

    pub fn contents(self) -> &'static str {
        match self {
            BookLanguage::English => "Contents",
            BookLanguage::Portuguese => "Sumário",
            BookLanguage::Spanish => "Índice",
            BookLanguage::French => "Table des matières",
        }
    }

    pub fn about_author(self) -> &'static str {
        match self {
            BookLanguage::English => "About the author",
            BookLanguage::Portuguese => "Sobre o autor",
            BookLanguage::Spanish => "Sobre el autor",
            BookLanguage::French => "À propos de l’auteur",
        }
    }

    /// "Also by {author}", where `{author}` is the project's author.
    pub fn also_by(self) -> &'static str {
        match self {
            BookLanguage::English => "Also by {author}",
            BookLanguage::Portuguese => "Outros livros de {author}",
            BookLanguage::Spanish => "Otros libros de {author}",
            BookLanguage::French => "Du même auteur",
        }
    }

    /// "Also by" when the project has no author.
    fn also_by_no_author(self) -> &'static str {
        match self {
            BookLanguage::English => "Also by the author",
            BookLanguage::Portuguese => "Outros livros do autor",
            BookLanguage::Spanish => "Otros libros del autor",
            BookLanguage::French => "Du même auteur",
        }
    }

    pub fn copyright(self) -> &'static str {
        match self {
            BookLanguage::English => "Copyright © {year} {author}. All rights reserved.",
            BookLanguage::Portuguese => "Copyright © {year} {author}. Todos os direitos reservados.",
            BookLanguage::Spanish => "Copyright © {year} {author}. Todos los derechos reservados.",
            BookLanguage::French => "Copyright © {year} {author}. Tous droits réservés.",
        }
    }

    /// A number up to ninety-nine in words, with a capital; larger ones stay
    /// digits.
    pub fn number_words(self, number: usize) -> String {
        let words = match self {
            BookLanguage::English => english_number(number),
            BookLanguage::Portuguese => portuguese_number(number),
            BookLanguage::Spanish => spanish_number(number),
            BookLanguage::French => french_number(number),
        };
        let Some(words) = words else {
            return number.to_string();
        };
        let mut chars = words.chars();
        chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
    }
}

fn english_number(number: usize) -> Option<String> {
    const ONES: [&str; 20] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
        "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    Some(match number {
        0..=19 => ONES[number].to_string(),
        20..=99 if number % 10 == 0 => TENS[number / 10].to_string(),
        20..=99 => format!("{}-{}", TENS[number / 10], ONES[number % 10]),
        _ => return None,
    })
}

/// Brazilian spelling: "dezesseis", "vinte e um".
fn portuguese_number(number: usize) -> Option<String> {
    const ONES: [&str; 20] = [
        "zero", "um", "dois", "três", "quatro", "cinco", "seis", "sete", "oito", "nove", "dez", "onze", "doze", "treze",
        "catorze", "quinze", "dezesseis", "dezessete", "dezoito", "dezenove",
    ];
    const TENS: [&str; 10] =
        ["", "", "vinte", "trinta", "quarenta", "cinquenta", "sessenta", "setenta", "oitenta", "noventa"];
    Some(match number {
        0..=19 => ONES[number].to_string(),
        20..=99 if number % 10 == 0 => TENS[number / 10].to_string(),
        20..=99 => format!("{} e {}", TENS[number / 10], ONES[number % 10]),
        _ => return None,
    })
}

fn spanish_number(number: usize) -> Option<String> {
    const ONES: [&str; 30] = [
        "cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez", "once", "doce", "trece",
        "catorce", "quince", "dieciséis", "diecisiete", "dieciocho", "diecinueve", "veinte", "veintiuno", "veintidós",
        "veintitrés", "veinticuatro", "veinticinco", "veintiséis", "veintisiete", "veintiocho", "veintinueve",
    ];
    const TENS: [&str; 10] = ["", "", "veinte", "treinta", "cuarenta", "cincuenta", "sesenta", "setenta", "ochenta", "noventa"];
    Some(match number {
        0..=29 => ONES[number].to_string(),
        30..=99 if number % 10 == 0 => TENS[number / 10].to_string(),
        30..=99 => format!("{} y {}", TENS[number / 10], ONES[number % 10]),
        _ => return None,
    })
}

/// "vingt et un", "soixante-dix", "quatre-vingts", "quatre-vingt-dix-neuf".
fn french_number(number: usize) -> Option<String> {
    const ONES: [&str; 20] = [
        "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze", "douze", "treize",
        "quatorze", "quinze", "seize", "dix-sept", "dix-huit", "dix-neuf",
    ];
    const TENS: [&str; 7] = ["", "", "vingt", "trente", "quarante", "cinquante", "soixante"];
    Some(match number {
        0..=19 => ONES[number].to_string(),
        // Seventy and ninety count on from sixty and eighty by the teens.
        20..=79 => {
            let (ten, rest) = if number >= 60 { (6, number - 60) } else { (number / 10, number % 10) };
            match rest {
                0 => TENS[ten].to_string(),
                1 | 11 => format!("{} et {}", TENS[ten], ONES[rest]),
                _ => format!("{}-{}", TENS[ten], ONES[rest]),
            }
        }
        80 => "quatre-vingts".to_string(),
        81..=99 => format!("quatre-vingt-{}", ONES[number - 80]),
        _ => return None,
    })
}

/// What sets a scene off from the one before it in the same chapter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneSeparator {
    /// A centered "* * *".
    #[default]
    Break,
    /// An empty line.
    Blank,
    /// The scene's title as a heading.
    Title,
    /// Nothing: the scenes run on.
    Nothing,
}

impl SceneSeparator {
    pub const ALL: [SceneSeparator; 4] =
        [SceneSeparator::Break, SceneSeparator::Blank, SceneSeparator::Title, SceneSeparator::Nothing];

    pub fn label(self) -> &'static str {
        match self {
            SceneSeparator::Break => "A scene break (* * *)",
            SceneSeparator::Blank => "An empty line",
            SceneSeparator::Title => "The scene's title",
            SceneSeparator::Nothing => "Nothing",
        }
    }
}

pub const FONT_SIZE_RANGE: (u32, u32) = (8, 24);
/// Margins in tenths of an inch.
pub const MARGIN_RANGE: (u32, u32) = (2, 30);
const NAME_MAX: usize = 60;
const FONT_MAX: usize = 60;
const WORD_MAX: usize = 24;
/// The copyright line and the dedication.
const LINE_MAX: usize = 400;
/// About the author and the list of other books.
const PASSAGE_MAX: usize = 4000;
const HEADING_MAX: usize = 80;
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
    /// How a chapter's heading reads.
    #[serde(default)]
    pub chapters: HeadingStyle,
    /// How a part's heading reads.
    #[serde(default)]
    pub parts: HeadingStyle,
    /// The word in front of a chapter's number, in the book's language.
    #[serde(default = "default_chapter_word")]
    pub chapter_word: String,
    #[serde(default = "default_part_word")]
    pub part_word: String,
    /// What sets one scene off from the next in a chapter.
    #[serde(default)]
    pub scenes: SceneSeparator,
    /// The pages before and after the text, other than the title page.
    #[serde(default)]
    pub matter: Matter,
    /// The language the book is printed in.
    #[serde(default)]
    pub language: BookLanguage,
}

/// The front and back matter a profile puts around the text. Each page is on
/// or off, and the text of each is kept even while it is off. Matter is made
/// at compile time: it is not a scene and counts toward no word target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Matter {
    pub copyright: bool,
    /// One line. `{year}` becomes the year of the compile, `{author}` the
    /// project's author.
    pub copyright_text: String,
    pub dedication: bool,
    pub dedication_text: String,
    /// A table of contents from the compiled parts and chapters.
    pub contents: bool,
    pub about_author: bool,
    pub about_author_text: String,
    pub also_by: bool,
    /// The writer's other books, one to a line.
    pub also_by_text: String,
    /// The headings of the matter pages. Empty means the book language's.
    pub contents_heading: String,
    pub about_author_heading: String,
    /// `{author}` becomes the project's author.
    pub also_by_heading: String,
}

impl Default for Matter {
    fn default() -> Self {
        Self {
            copyright: false,
            copyright_text: BookLanguage::English.copyright().into(),
            dedication: false,
            dedication_text: String::new(),
            contents: false,
            about_author: false,
            about_author_text: String::new(),
            also_by: false,
            also_by_text: String::new(),
            contents_heading: String::new(),
            about_author_heading: String::new(),
            also_by_heading: String::new(),
        }
    }
}

impl Matter {
    /// The copyright line with the year and the author filled in.
    pub fn copyright_line(&self, year: i32, author: &str) -> String {
        let line = self.copyright_text.replace("{year}", &year.to_string()).replace("{author}", author.trim());
        // An empty author leaves "2026 ." behind; tidy the space it leaves.
        line.split_whitespace().collect::<Vec<_>>().join(" ").replace(" .", ".").replace(" ,", ",")
    }

    /// The titles listed under "Also by", without empty lines.
    pub fn also_by_titles(&self) -> Vec<String> {
        self.also_by_text.lines().map(str::trim).filter(|line| !line.is_empty()).map(String::from).collect()
    }

    fn normalized(mut self) -> Self {
        self.copyright_text = clean_text(&self.copyright_text.replace(['\r', '\n', '\t'], " "), LINE_MAX);
        self.dedication_text = clean_lines(&self.dedication_text, LINE_MAX);
        self.about_author_text = clean_lines(&self.about_author_text, PASSAGE_MAX);
        self.also_by_text = clean_lines(&self.also_by_text, PASSAGE_MAX);
        self.contents_heading = clean_text(&self.contents_heading, HEADING_MAX);
        self.about_author_heading = clean_text(&self.about_author_heading, HEADING_MAX);
        self.also_by_heading = clean_text(&self.also_by_heading, HEADING_MAX);
        self
    }
}

fn default_chapter_word() -> String {
    "Chapter".into()
}

fn default_part_word() -> String {
    "Part".into()
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
            chapters: HeadingStyle::NumberAndTitle,
            parts: HeadingStyle::NumberAndTitle,
            chapter_word: default_chapter_word(),
            part_word: default_part_word(),
            scenes: SceneSeparator::Break,
            matter: Matter::default(),
            language: BookLanguage::English,
        },
        CompileProfile {
            id: fixed_id(PLAIN_TEXT_ID),
            name: "Plain text".into(),
            format: CompileFormat::Markdown,
            title_page: true,
            page: PageSetup::default(),
            chapters: HeadingStyle::NumberAndTitle,
            parts: HeadingStyle::NumberAndTitle,
            chapter_word: default_chapter_word(),
            part_word: default_part_word(),
            scenes: SceneSeparator::Break,
            matter: Matter::default(),
            language: BookLanguage::English,
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
        self.chapter_word = clean_text(&self.chapter_word, WORD_MAX);
        self.part_word = clean_text(&self.part_word, WORD_MAX);
        self.matter = self.matter.normalized();
        self
    }

    pub fn is_built_in(&self) -> bool {
        built_in_profile(self.id).is_some()
    }

    /// Print the book in `language`. The words before a number and the
    /// copyright line follow it when they are still the old language's own;
    /// a word or line the writer changed is kept.
    pub fn set_language(&mut self, language: BookLanguage) {
        let old = self.language;
        if self.chapter_word == old.chapter_word() {
            self.chapter_word = language.chapter_word().into();
        }
        if self.part_word == old.part_word() {
            self.part_word = language.part_word().into();
        }
        if self.matter.copyright_text == old.copyright() {
            self.matter.copyright_text = language.copyright().into();
        }
        self.language = language;
    }

    /// The table of contents' heading.
    pub fn contents_heading(&self) -> String {
        or_default(&self.matter.contents_heading, self.language.contents())
    }

    pub fn about_author_heading(&self) -> String {
        or_default(&self.matter.about_author_heading, self.language.about_author())
    }

    /// "Also by", with the author filled in, or the language's words for
    /// "the author" when the project has none.
    pub fn also_by_heading(&self, author: &str) -> String {
        let author = author.trim();
        let own = self.matter.also_by_heading.trim();
        if own.is_empty() && author.is_empty() {
            return self.language.also_by_no_author().into();
        }
        let heading = if own.is_empty() { self.language.also_by() } else { own };
        heading.replace("{author}", author).split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

pub(crate) fn clean_text(text: &str, max: usize) -> String {
    text.chars().filter(|ch| !ch.is_control()).collect::<String>().trim().chars().take(max).collect()
}

fn or_default(own: &str, default: &str) -> String {
    if own.trim().is_empty() { default.into() } else { own.trim().into() }
}

/// Text that keeps its line breaks: each line cleaned, Windows line ends
/// made plain, and the whole cut to `max` characters.
fn clean_lines(text: &str, max: usize) -> String {
    let lines: Vec<String> = text.lines().map(|line| clean_text(line, max)).collect();
    lines.join("\n").trim().chars().take(max).collect()
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
            chapter_word: "  Capítulo	".into(),
            ..built_in_profiles().remove(0)
        }
        .normalized();
        assert_eq!(wild.name, "Badname");
        assert_eq!(wild.page.font, "Times New Roman");
        assert_eq!(wild.page.font_size, FONT_SIZE_RANGE.1);
        assert_eq!(wild.page.margin, MARGIN_RANGE.0);
        assert_eq!(wild.chapter_word, "Capítulo", "a word in the book's own language");
        assert!(!wild.is_built_in());
    }

    #[test]
    fn a_profile_from_a_file_with_only_a_name_and_id_has_defaults() {
        let json = r#"{"id":"6a0c1f3e-2b7d-4c4e-9a51-0d8e6b1f7c99","name":"Mine"}"#;
        let profile: CompileProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.format, CompileFormat::Docx);
        assert!(profile.title_page);
        assert_eq!(profile.page, PageSetup::default());
        assert_eq!(profile.chapters, HeadingStyle::NumberAndTitle);
        assert_eq!(profile.chapter_word, "Chapter");
        assert_eq!(profile.scenes, SceneSeparator::Break);
    }

    #[test]
    fn matter_from_a_file_keeps_its_lines_and_loses_control_characters() {
        let mut profile = built_in_profiles().remove(0);
        assert_eq!(profile.matter, Matter::default(), "no matter but the title page until the writer asks");
        profile.matter.about_author_text = "Maya\u{7} lives by the sea.\r\n\r\nShe writes at night.  ".into();
        profile.matter.copyright_text = "© {year}\n{author}".into();
        profile.matter.also_by_text = "x".repeat(PASSAGE_MAX + 10);
        let matter = profile.normalized().matter;
        assert_eq!(matter.about_author_text, "Maya lives by the sea.\n\nShe writes at night.");
        assert_eq!(matter.copyright_line(2026, "Maya Reis"), "© 2026 Maya Reis", "the copyright is one line");
        assert_eq!(matter.also_by_text.chars().count(), PASSAGE_MAX);
        assert_eq!(Matter::default().copyright_line(2026, ""), "Copyright © 2026. All rights reserved.");
    }

    #[test]
    fn number_words_and_headings_follow_the_book_language() {
        let words = |language: BookLanguage, numbers: &[usize]| {
            numbers.iter().map(|n| language.number_words(*n)).collect::<Vec<_>>()
        };
        assert_eq!(words(BookLanguage::English, &[1, 21, 40, 100]), ["One", "Twenty-one", "Forty", "100"]);
        assert_eq!(words(BookLanguage::Portuguese, &[2, 16, 21, 99]), ["Dois", "Dezesseis", "Vinte e um", "Noventa e nove"]);
        assert_eq!(words(BookLanguage::Spanish, &[16, 21, 22, 31]), ["Dieciséis", "Veintiuno", "Veintidós", "Treinta y uno"]);
        assert_eq!(
            words(BookLanguage::French, &[17, 21, 70, 71, 77, 80, 81, 91, 99]),
            [
                "Dix-sept", "Vingt et un", "Soixante-dix", "Soixante et onze", "Soixante-dix-sept", "Quatre-vingts",
                "Quatre-vingt-un", "Quatre-vingt-onze", "Quatre-vingt-dix-neuf"
            ]
        );

        let mut profile = built_in_profiles().remove(0);
        profile.part_word = "Livro".into();
        profile.set_language(BookLanguage::Portuguese);
        assert_eq!(profile.chapter_word, "Capítulo", "the language's own word follows it");
        assert_eq!(profile.part_word, "Livro", "a word the writer chose is kept");
        assert_eq!(profile.matter.copyright_text, BookLanguage::Portuguese.copyright());
        assert_eq!(
            profile.chapters.heading(profile.language, &profile.chapter_word, 2, "O Porto"),
            "Capítulo 2: O Porto"
        );
        assert_eq!(HeadingStyle::NumberWords.heading(profile.language, "Capítulo", 3, ""), "Capítulo Três");
        assert_eq!(profile.contents_heading(), "Sumário");
        assert_eq!(profile.also_by_heading("Maya Reis"), "Outros livros de Maya Reis");
        assert_eq!(profile.also_by_heading(""), "Outros livros do autor");
        profile.matter.contents_heading = " Índice geral ".into();
        profile.matter.also_by_heading = "Da mesma autora".into();
        assert_eq!(profile.contents_heading(), "Índice geral");
        assert_eq!(profile.also_by_heading("Maya Reis"), "Da mesma autora");
        let json = r#"{"id":"6a0c1f3e-2b7d-4c4e-9a51-0d8e6b1f7c99","name":"Older"}"#;
        assert_eq!(serde_json::from_str::<CompileProfile>(json).unwrap().language, BookLanguage::English);
    }

    #[test]
    fn chapter_headings_read_as_the_profile_says_without_repeating_a_placeholder_title() {
        let style = HeadingStyle::NumberAndTitle;
        assert_eq!(style.heading(BookLanguage::English, "Chapter", 3, "The Harbor"), "Chapter 3: The Harbor");
        assert_eq!(style.heading(BookLanguage::English, "Chapter", 2, "Chapter 2"), "Chapter 2", "a new chapter's own name");
        assert_eq!(style.heading(BookLanguage::English, "Chapter", 2, ""), "Chapter 2");
        assert_eq!(style.heading(BookLanguage::English, "Chapter", 2, "Chapters of the sea"), "Chapter 2: Chapters of the sea");
        assert_eq!(HeadingStyle::Number.heading(BookLanguage::English, "Part", 4, "Inland"), "Part 4");
        assert_eq!(HeadingStyle::NumberWords.heading(BookLanguage::English, "Chapter", 21, "x"), "Chapter Twenty-one");
        assert_eq!(HeadingStyle::NumberWords.heading(BookLanguage::English, "Chapter", 140, "x"), "Chapter 140");
        assert_eq!(HeadingStyle::Title.heading(BookLanguage::English, "Chapter", 5, "The Ridge"), "The Ridge");
        assert_eq!(HeadingStyle::Title.heading(BookLanguage::English, "Chapter", 5, "Chapter 5"), "Chapter 5");
        assert_eq!(HeadingStyle::Number.heading(BookLanguage::English, "", 7, "x"), "7", "no word, only the number");
    }
}
